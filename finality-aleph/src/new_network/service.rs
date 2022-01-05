use crate::new_network::{
    ConnectionCommand, Data, DataCommand, Network, PeerId, Protocol, ALEPH_PROTOCOL_NAME,
    ALEPH_VALIDATOR_PROTOCOL_NAME,
};
use futures::{channel::mpsc, StreamExt};
use log::{debug, error, trace};
use sc_network::{multiaddr, Event};
use std::{
    borrow::Cow,
    collections::{HashSet, VecDeque},
    iter,
};

struct Service<N: Network, D: Data> {
    network: N,
    messages_from_user: mpsc::UnboundedReceiver<(D, DataCommand)>,
    messages_for_user: mpsc::UnboundedSender<D>,
    commands_from_manager: mpsc::UnboundedReceiver<ConnectionCommand>,
    connected_peers: HashSet<PeerId>,
    to_send: VecDeque<(D, PeerId, Protocol)>,
}

pub struct IO<D: Data> {
    messages_from_user: mpsc::UnboundedReceiver<(D, DataCommand)>,
    messages_for_user: mpsc::UnboundedSender<D>,
    commands_from_manager: mpsc::UnboundedReceiver<ConnectionCommand>,
}

impl<N: Network, D: Data> Service<N, D> {
    pub fn new(network: N, io: IO<D>) -> Service<N, D> {
        let IO {
            messages_from_user,
            messages_for_user,
            commands_from_manager,
        } = io;
        Service {
            network,
            messages_from_user,
            messages_for_user,
            commands_from_manager,
            connected_peers: HashSet::new(),
            to_send: VecDeque::new(),
        }
    }

    fn send_to_peer(&mut self, data: D, peer: PeerId, protocol: Protocol) {
        self.to_send.push_back((data, peer, protocol));
    }

    fn broadcast(&mut self, data: D) {
        for peer in self.connected_peers.clone() {
            // We only broadcast authentication information in this sense, so we use the generic
            // Protocol.
            self.send_to_peer(data.clone(), peer, Protocol::Generic);
        }
    }

    fn handle_network_event(&mut self, event: Event) -> Result<(), mpsc::TrySendError<D>> {
        match event {
            Event::SyncConnected { remote } => {
                trace!(target: "aleph-network", "SyncConnected event for peer {:?}", remote);
                let addr = iter::once(multiaddr::Protocol::P2p(remote.into())).collect();
                self.network.add_reserved(
                    iter::once(addr).collect(),
                    Cow::Borrowed(ALEPH_PROTOCOL_NAME),
                );
            }
            Event::SyncDisconnected { remote } => {
                trace!(target: "aleph-network", "SyncDisconnected event for peer {:?}", remote);
                self.network.remove_reserved(
                    iter::once(remote.into()).collect(),
                    Cow::Borrowed(ALEPH_PROTOCOL_NAME),
                );
            }
            Event::NotificationStreamOpened {
                remote, protocol, ..
            } => {
                if protocol == ALEPH_PROTOCOL_NAME {
                    self.connected_peers.insert(remote.into());
                }
            }
            Event::NotificationStreamClosed { remote, protocol } => {
                if protocol == ALEPH_PROTOCOL_NAME {
                    self.connected_peers.remove(&remote.into());
                }
            }
            Event::NotificationsReceived {
                remote: _,
                messages,
            } => {
                for (protocol, data) in messages.into_iter() {
                    if protocol == ALEPH_PROTOCOL_NAME || protocol == ALEPH_VALIDATOR_PROTOCOL_NAME
                    {
                        match D::decode(&mut &data[..]) {
                            Ok(message) => self.messages_for_user.unbounded_send(message)?,
                            Err(e) => {
                                debug!(target: "aleph-network", "Error decoding message: {}", e)
                            }
                        }
                    }
                }
            }
            // Irrelevant for us, ignore.
            Event::Dht(_) => {}
        }
        Ok(())
    }

    fn on_manager_command(&self, command: ConnectionCommand) {
        use ConnectionCommand::*;
        match command {
            AddReserved(addresses) => self
                .network
                .add_reserved(addresses, Cow::Borrowed(ALEPH_VALIDATOR_PROTOCOL_NAME)),
            DelReserved(peers) => self
                .network
                .remove_reserved(peers, Cow::Borrowed(ALEPH_VALIDATOR_PROTOCOL_NAME)),
        }
    }

    fn on_user_command(&mut self, data: D, command: DataCommand) {
        use DataCommand::*;
        match command {
            Broadcast => self.broadcast(data),
            SendTo(peer, protocol) => self.send_to_peer(data, peer, protocol),
        }
    }

    async fn send(
        network: &N,
        send_queue: &mut VecDeque<(D, PeerId, Protocol)>,
    ) -> Option<Result<(), N::SendError>> {
        // We should not pop send_queue here. Using `send_queue.front()` is intended.
        // Send is asynchronous, so it might happen that we pop data here and then
        // `network.send` does not finish and gets cancelled. So in this case we would
        // lose a popped message.
        let (data, peer, protocol) = send_queue.front()?;
        let result = network.send(data.encode(), *peer, protocol.name()).await;
        send_queue.pop_front();
        Some(result)
    }

    pub async fn run(mut self) {
        let mut events_from_network = self.network.event_stream();
        loop {
            tokio::select! {
                maybe_event = events_from_network.next() => match maybe_event {
                    Some(event) => if let Err(e) = self.handle_network_event(event) {
                        error!(target: "aleph-network", "Cannot forward messages to user: {:?}", e);
                        return;
                    },
                    None => {
                        error!(target: "aleph-network", "Network event stream ended.");
                        return;
                    }
                },
                maybe_command = self.commands_from_manager.next() => match maybe_command {
                    Some(command) => self.on_manager_command(command),
                    None => {
                        error!(target: "aleph-network", "Manager command stream ended.");
                        return;
                    }
                },
                maybe_message = self.messages_from_user.next() => match maybe_message {
                    Some((data, command)) => self.on_user_command(data, command),
                    None => {
                        error!(target: "aleph-network", "User message stream ended.");
                        return;
                    }
                },
                Some(result) = Self::send(&self.network, &mut self.to_send) => {
                    if let Err(e) = result {
                        debug!(target: "aleph-network", "Failed sending data to peer: {:?}", e);
                    }
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ConnectionCommand, DataCommand, Service, IO};
    use crate::new_network::{
        manager::testing::MockNetworkIdentity,
        mock::{MockNetwork, MockSendError},
        NetworkIdentity, Protocol, ALEPH_PROTOCOL_NAME, ALEPH_VALIDATOR_PROTOCOL_NAME,
    };
    use codec::Encode;
    use futures::{
        channel::{mpsc, oneshot},
        Future, StreamExt,
    };
    use sc_network::{
        multiaddr::Protocol as ScProtocol, Event, Multiaddr as ScMultiaddr, ObservedRole,
    };
    use std::{borrow::Cow, collections::HashSet, iter, iter::FromIterator};

    type MockData = Vec<u8>;

    pub struct MockIO {
        messages_for_user: mpsc::UnboundedSender<(MockData, DataCommand)>,
        messages_from_user: mpsc::UnboundedReceiver<MockData>,
        commands_for_manager: mpsc::UnboundedSender<ConnectionCommand>,
    }

    async fn prepare() -> (
        impl Future<Output = sc_service::Result<(), tokio::task::JoinError>>,
        oneshot::Sender<()>,
        MockNetwork,
        MockIO,
    ) {
        let (mock_messages_for_user, messages_from_user) = mpsc::unbounded();
        let (messages_for_user, mock_messages_from_user) = mpsc::unbounded();
        let (mock_commands_for_manager, commands_from_manager) = mpsc::unbounded();
        let io = IO {
            messages_from_user,
            messages_for_user,
            commands_from_manager,
        };
        let mock_io = MockIO {
            messages_for_user: mock_messages_for_user,
            messages_from_user: mock_messages_from_user,
            commands_for_manager: mock_commands_for_manager,
        };

        let (event_stream_oneshot_tx, event_stream_oneshot_rx) = oneshot::channel();
        let network = MockNetwork::new(event_stream_oneshot_tx);
        let service = Service::new(network.clone(), io);

        let (exit_tx, exit_rx) = oneshot::channel();
        let task_handle = async move {
            tokio::select! {
                _ = service.run() => { },
                _ = exit_rx => { },
            };
        };
        let service_handle = tokio::spawn(task_handle);

        // wait till service takes the event_stream
        event_stream_oneshot_rx.await.unwrap();

        (service_handle, exit_tx, network, mock_io)
    }

    #[tokio::test]
    async fn test_sync_connected() {
        let (service_handle, exit_tx, mut network, _mock_io) = prepare().await;

        let identity = MockNetworkIdentity::new().identity();

        network.emit_event(Event::SyncConnected {
            remote: identity.1.into(),
        });

        let expected = (
            iter::once(ScMultiaddr::empty().with(ScProtocol::P2p(identity.1 .0.into()))).collect(),
            Cow::Borrowed(ALEPH_PROTOCOL_NAME),
        );

        assert_eq!(
            network
                .add_reserved
                .1
                .lock()
                .next()
                .await
                .expect("Should receive message"),
            expected
        );

        exit_tx.send(()).ok();
        service_handle.await.unwrap();
        network.close_channels();
    }

    #[tokio::test]
    async fn test_sync_disconnected() {
        let (service_handle, exit_tx, mut network, _mock_io) = prepare().await;

        let identity = MockNetworkIdentity::new().identity();

        network.emit_event(Event::SyncDisconnected {
            remote: identity.1.into(),
        });

        let expected = (
            iter::once(identity.1).collect(),
            Cow::Borrowed(ALEPH_PROTOCOL_NAME),
        );

        assert_eq!(
            network
                .remove_reserved
                .1
                .lock()
                .next()
                .await
                .expect("Should receive message"),
            expected
        );

        exit_tx.send(()).ok();
        service_handle.await.unwrap();
        network.close_channels();
    }

    #[tokio::test]
    async fn test_notification_stream_opened() {
        let (service_handle, exit_tx, mut network, mock_io) = prepare().await;

        let identities: Vec<_> = (0..3)
            .map(|_| MockNetworkIdentity::new().identity())
            .collect();

        identities.iter().for_each(|identity| {
            network.emit_event(Event::NotificationStreamOpened {
                protocol: Cow::Borrowed(ALEPH_PROTOCOL_NAME),
                remote: identity.1.into(),
                negotiated_fallback: None,
                role: ObservedRole::Authority,
            })
        });

        // We do this only to make sure that NotificationStreamOpened events are handled
        network.emit_event(Event::SyncConnected {
            remote: identities[0].1.into(),
        });
        let expected = ScMultiaddr::empty().with(ScProtocol::P2p(identities[0].1 .0.into()));
        assert_eq!(
            network
                .add_reserved
                .1
                .lock()
                .next()
                .await
                .expect("Should receive message"),
            (
                iter::once(expected).collect(),
                Cow::Borrowed(ALEPH_PROTOCOL_NAME)
            )
        );

        let message: Vec<u8> = vec![1, 2, 3];
        mock_io
            .messages_for_user
            .unbounded_send((message.clone(), DataCommand::Broadcast))
            .ok();

        let broadcasted_messages = HashSet::<_>::from_iter(
            network
                .send_message
                .1
                .lock()
                .by_ref()
                .take(identities.len())
                .collect::<Vec<_>>()
                .await
                .iter()
                .cloned(),
        );

        let expected_messages = HashSet::from_iter(identities.iter().map(|identity| {
            (
                message.encode(),
                identity.1,
                Cow::Borrowed(ALEPH_PROTOCOL_NAME),
            )
        }));

        assert_eq!(broadcasted_messages, expected_messages);

        exit_tx.send(()).ok();
        service_handle.await.unwrap();
        network.close_channels();
    }

    #[tokio::test]
    async fn test_notification_stream_closed() {
        let (service_handle, exit_tx, mut network, mock_io) = prepare().await;

        let identities: Vec<_> = (0..4)
            .map(|_| MockNetworkIdentity::new().identity())
            .collect();
        let opened_authorities_n = 2;

        identities.iter().for_each(|identity| {
            network.emit_event(Event::NotificationStreamOpened {
                protocol: Cow::Borrowed(ALEPH_PROTOCOL_NAME),
                remote: identity.1.into(),
                negotiated_fallback: None,
                role: ObservedRole::Authority,
            })
        });

        identities
            .iter()
            .skip(opened_authorities_n)
            .for_each(|identity| {
                network.emit_event(Event::NotificationStreamClosed {
                    protocol: Cow::Borrowed(ALEPH_PROTOCOL_NAME),
                    remote: identity.1.into(),
                })
            });

        // We do this only to make sure that NotificationStreamOpened events are handled
        network.emit_event(Event::SyncConnected {
            remote: identities[0].1.into(),
        });
        let expected = ScMultiaddr::empty().with(ScProtocol::P2p(identities[0].1 .0.into()));
        assert_eq!(
            network
                .add_reserved
                .1
                .lock()
                .next()
                .await
                .expect("Should receive message"),
            (
                iter::once(expected).collect(),
                Cow::Borrowed(ALEPH_PROTOCOL_NAME)
            )
        );

        let messages: Vec<Vec<u8>> = vec![vec![1, 2, 3], vec![4, 5, 6]];
        messages.iter().for_each(|m| {
            mock_io
                .messages_for_user
                .unbounded_send((m.clone(), DataCommand::Broadcast))
                .ok();
        });

        let broadcasted_messages = HashSet::<_>::from_iter(
            network
                .send_message
                .1
                .lock()
                .by_ref()
                .take(opened_authorities_n * messages.len())
                .collect::<Vec<_>>()
                .await
                .iter()
                .cloned(),
        );

        let expected_messages = HashSet::from_iter(
            identities
                .iter()
                .take(opened_authorities_n)
                .map(|identity| {
                    messages
                        .iter()
                        .map(move |m| (m.encode(), identity.1, Cow::Borrowed(ALEPH_PROTOCOL_NAME)))
                })
                .flatten(),
        );

        assert_eq!(broadcasted_messages, expected_messages);

        exit_tx.send(()).ok();
        service_handle.await.unwrap();
        network.close_channels();
    }

    #[tokio::test]
    async fn test_data_command_send_to() {
        let (service_handle, exit_tx, network, mock_io) = prepare().await;

        let identity = MockNetworkIdentity::new().identity();

        let message: Vec<u8> = vec![1, 2, 3];

        mock_io
            .messages_for_user
            .unbounded_send((
                message.clone(),
                DataCommand::SendTo(identity.1, Protocol::Validator),
            ))
            .ok();

        let expected = (message.encode(), identity.1, Protocol::Validator.name());

        assert_eq!(
            network
                .send_message
                .1
                .lock()
                .next()
                .await
                .expect("Should receive message"),
            expected,
        );

        exit_tx.send(()).ok();
        service_handle.await.unwrap();
        network.close_channels();
    }

    #[tokio::test]
    async fn test_data_command_send_to_error() {
        let (service_handle, exit_tx, network, mock_io) = prepare().await;
        let all_authorities_n = 4;
        let closed_authorities_n = 2;
        for _ in 0..closed_authorities_n {
            network
                .network_errors
                .lock()
                .push_back(MockSendError::SomeError);
        }

        let identities: Vec<_> = (0..4)
            .map(|_| MockNetworkIdentity::new().identity())
            .collect();
        let message: Vec<u8> = vec![1, 2, 3];

        identities.iter().for_each(|identity| {
            mock_io
                .messages_for_user
                .unbounded_send((
                    message.clone(),
                    DataCommand::SendTo(identity.1, Protocol::Validator),
                ))
                .ok();
        });

        let broadcasted_messages = HashSet::<_>::from_iter(
            network
                .send_message
                .1
                .lock()
                .by_ref()
                .take(all_authorities_n - closed_authorities_n)
                .collect::<Vec<_>>()
                .await
                .iter()
                .cloned(),
        );

        let expected_messages = HashSet::from_iter(
            identities
                .iter()
                .skip(closed_authorities_n)
                .map(|identity| (message.encode(), identity.1, Protocol::Validator.name())),
        );

        assert_eq!(broadcasted_messages, expected_messages);

        exit_tx.send(()).ok();
        service_handle.await.unwrap();
        network.close_channels();
    }

    #[tokio::test]
    async fn test_notification_received() {
        let (service_handle, exit_tx, mut network, mut mock_io) = prepare().await;

        let identity = MockNetworkIdentity::new().identity();

        let message: Vec<u8> = vec![1, 2, 3];
        let incorrect_message: Vec<u8> = vec![4, 5, 6];

        network.emit_event(Event::NotificationsReceived {
            remote: identity.1.into(),
            messages: vec![(
                Cow::Borrowed("INCORRECT/PROTOCOL/NAME"),
                Vec::encode(&incorrect_message).into(),
            )],
        });

        network.emit_event(Event::NotificationsReceived {
            remote: identity.1.into(),
            messages: vec![(
                Cow::Borrowed(ALEPH_PROTOCOL_NAME),
                Vec::encode(&message).into(),
            )],
        });

        assert_eq!(
            mock_io
                .messages_from_user
                .next()
                .await
                .expect("Should receive message"),
            message
        );

        exit_tx.send(()).ok();
        service_handle.await.unwrap();
        network.close_channels();
    }

    #[tokio::test]
    async fn test_command_add_reserved() {
        let (service_handle, exit_tx, network, mock_io) = prepare().await;
        let identity = MockNetworkIdentity::new().identity();

        mock_io
            .commands_for_manager
            .unbounded_send(ConnectionCommand::AddReserved(
                identity.0.clone().into_iter().collect(),
            ))
            .ok();

        let expected = (
            identity.0.into_iter().collect(),
            Cow::Borrowed(ALEPH_VALIDATOR_PROTOCOL_NAME),
        );

        assert_eq!(
            network
                .add_reserved
                .1
                .lock()
                .next()
                .await
                .expect("Should receive message"),
            expected
        );

        exit_tx.send(()).ok();
        service_handle.await.unwrap();
        network.close_channels();
    }

    #[tokio::test]
    async fn test_command_remove_reserved() {
        let (service_handle, exit_tx, network, mock_io) = prepare().await;
        let identity = MockNetworkIdentity::new().identity();

        mock_io
            .commands_for_manager
            .unbounded_send(ConnectionCommand::DelReserved(
                iter::once(identity.1).collect(),
            ))
            .ok();

        let expected = (
            iter::once(identity.1).collect(),
            Cow::Borrowed(ALEPH_VALIDATOR_PROTOCOL_NAME),
        );

        assert_eq!(
            network
                .remove_reserved
                .1
                .lock()
                .next()
                .await
                .expect("Should receive message"),
            expected
        );

        exit_tx.send(()).ok();
        service_handle.await.unwrap();
        network.close_channels();
    }
}
