use crate::network::{
    ConnectionCommand, Data, DataCommand, Network, NetworkSender, PeerId, Protocol,
    ALEPH_PROTOCOL_NAME, ALEPH_VALIDATOR_PROTOCOL_NAME,
};
use futures::{channel::mpsc, StreamExt};
use log::{debug, error, trace, warn};
use sc_network::{multiaddr, Event};
use sc_service::SpawnTaskHandle;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    future::Future,
    iter,
};

pub struct Service<N: Network, D: Data> {
    network: N,
    messages_from_user: mpsc::UnboundedReceiver<(D, DataCommand)>,
    messages_for_user: mpsc::UnboundedSender<D>,
    commands_from_manager: mpsc::UnboundedReceiver<ConnectionCommand>,
    connected_peers: HashSet<PeerId>,
    peer_senders: HashMap<PeerId, mpsc::Sender<(D, Protocol)>>,
    spawn_handle: SpawnTaskHandle,
}

pub struct IO<D: Data> {
    messages_from_user: mpsc::UnboundedReceiver<(D, DataCommand)>,
    messages_for_user: mpsc::UnboundedSender<D>,
    commands_from_manager: mpsc::UnboundedReceiver<ConnectionCommand>,
}

impl<D: Data> IO<D> {
    pub fn new(
        messages_from_user: mpsc::UnboundedReceiver<(D, DataCommand)>,
        messages_for_user: mpsc::UnboundedSender<D>,
        commands_from_manager: mpsc::UnboundedReceiver<ConnectionCommand>,
    ) -> IO<D> {
        IO {
            messages_from_user,
            messages_for_user,
            commands_from_manager,
        }
    }
}
const PEER_BUFFER_SIZE: usize = 100;

impl<N: Network, D: Data> Service<N, D> {
    pub fn new(network: N, spawn_handle: SpawnTaskHandle, io: IO<D>) -> Service<N, D> {
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
            spawn_handle,
            connected_peers: HashSet::new(),
            peer_senders: HashMap::new(),
        }
    }

    fn peer_sender(
        &self,
        peer_id: PeerId,
        mut receiver: mpsc::Receiver<(D, Protocol)>,
    ) -> impl Future<Output = ()> + Send + 'static {
        let network = self.network.clone();
        async move {
            let mut senders: HashMap<Cow<'static, str>, N::NetworkSender> = HashMap::new();
            loop {
                if let Some((data, protocol)) = receiver.next().await {
                    let sender = if let Some(sender) = senders.get(&protocol.name()) {
                        sender
                    } else {
                        match network.sender(peer_id, protocol.name()) {
                            Ok(sender) => senders.entry(protocol.name()).or_insert(sender),
                            Err(e) => {
                                debug!(target: "aleph-network", "Failed creating sender. Dropping message: {:?}", e);
                                continue;
                            }
                        }
                    };
                    if let Err(e) = sender.send(data.encode()).await {
                        debug!(target: "aleph-network", "Failed sending data to peer. Dropping sender and message: {:?}", e);
                        senders.remove(&protocol.name());
                    }
                } else {
                    debug!(target: "aleph-network", "Sender was dropped for peer {:?}. Peer sender exiting.", peer_id);
                    return;
                }
            }
        }
    }

    fn send_to_peer(&mut self, data: D, peer: PeerId, protocol: Protocol) {
        if let Some(sender) = self.peer_senders.get_mut(&peer) {
            if let Err(e) = sender.try_send((data, protocol)) {
                if e.is_full() {
                    warn!(target: "aleph-network", "Failed sending data to peer because buffer is full: {:?}", peer);
                }
                // Receiver can also be dropped when thread cannot send to peer. In case receiver is dropped this entry will be removed by Event::NotificationStreamClosed
                // No need to remove the entry here
                if e.is_disconnected() {
                    trace!(target: "aleph-network", "Failed sending data to peer because peer_sender receiver is dropped: {:?}", peer);
                }
            }
        }
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
                    let (tx, rx) = mpsc::channel(PEER_BUFFER_SIZE);
                    self.spawn_handle.spawn(
                        "aleph/network/peer_sender",
                        None,
                        self.peer_sender(remote.into(), rx),
                    );
                    self.connected_peers.insert(remote.into());
                    self.peer_senders.insert(remote.into(), tx);
                }
            }
            Event::NotificationStreamClosed { remote, protocol } => {
                if protocol == ALEPH_PROTOCOL_NAME {
                    self.connected_peers.remove(&remote.into());
                    self.peer_senders.remove(&remote.into());
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
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ConnectionCommand, DataCommand, Service, IO};
    use crate::network::{
        manager::testing::MockNetworkIdentity,
        mock::{MockNetwork, MockSenderError},
        NetworkIdentity, Protocol, ALEPH_PROTOCOL_NAME, ALEPH_VALIDATOR_PROTOCOL_NAME,
    };
    use codec::Encode;
    use futures::{
        channel::{mpsc, oneshot},
        StreamExt,
    };
    use sc_network::{
        multiaddr::Protocol as ScProtocol, Event, Multiaddr as ScMultiaddr, ObservedRole,
    };
    use sc_service::TaskManager;
    use std::{borrow::Cow, collections::HashSet, iter, iter::FromIterator};
    use tokio::runtime::Handle;
    use tokio::task::JoinHandle;

    type MockData = Vec<u8>;

    pub struct MockIO {
        messages_for_user: mpsc::UnboundedSender<(MockData, DataCommand)>,
        messages_from_user: mpsc::UnboundedReceiver<MockData>,
        commands_for_manager: mpsc::UnboundedSender<ConnectionCommand>,
    }

    pub struct TestData {
        pub service_handle: JoinHandle<()>,
        pub exit_tx: oneshot::Sender<()>,
        pub network: MockNetwork,
        pub mock_io: MockIO,
        // `TaskManager` can't be dropped for `SpawnTaskHandle` to work
        _task_manager: TaskManager,
    }

    impl TestData {
        async fn prepare() -> Self {
            let task_manager = TaskManager::new(Handle::current(), None).unwrap();

            // Prepare communication with service
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
            // Prepare service
            let (event_stream_oneshot_tx, event_stream_oneshot_rx) = oneshot::channel();
            let network = MockNetwork::new(event_stream_oneshot_tx);
            let service = Service::new(network.clone(), task_manager.spawn_handle(), io);
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

            // `TaskManager` needs to be passed.
            Self {
                service_handle,
                exit_tx,
                network,
                mock_io,
                _task_manager: task_manager,
            }
        }

        async fn cleanup(self) {
            self.exit_tx.send(()).unwrap();
            self.service_handle.await.unwrap();
            self.network.close_channels();
        }

        // We do this only to make sure that NotificationStreamOpened/NotificationStreamClosed events are handled
        async fn wait_for_events_handled(&mut self) {
            let identity = MockNetworkIdentity::new().identity();
            self.network.emit_event(Event::SyncConnected {
                remote: identity.1.into(),
            });
            let expected = ScMultiaddr::empty().with(ScProtocol::P2p(identity.1 .0.into()));
            assert_eq!(
                self.network
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
        }
    }

    #[tokio::test]
    async fn test_sync_connected() {
        let mut test_data = TestData::prepare().await;

        let identity = MockNetworkIdentity::new().identity();

        test_data.network.emit_event(Event::SyncConnected {
            remote: identity.1.into(),
        });

        let expected = (
            iter::once(ScMultiaddr::empty().with(ScProtocol::P2p(identity.1 .0.into()))).collect(),
            Cow::Borrowed(ALEPH_PROTOCOL_NAME),
        );

        assert_eq!(
            test_data
                .network
                .add_reserved
                .1
                .lock()
                .next()
                .await
                .expect("Should receive message"),
            expected
        );

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_sync_disconnected() {
        let mut test_data = TestData::prepare().await;

        let identity = MockNetworkIdentity::new().identity();

        test_data.network.emit_event(Event::SyncDisconnected {
            remote: identity.1.into(),
        });

        let expected = (
            iter::once(identity.1).collect(),
            Cow::Borrowed(ALEPH_PROTOCOL_NAME),
        );

        assert_eq!(
            test_data
                .network
                .remove_reserved
                .1
                .lock()
                .next()
                .await
                .expect("Should receive message"),
            expected
        );

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_notification_stream_opened() {
        let mut test_data = TestData::prepare().await;

        let identities: Vec<_> = (0..3)
            .map(|_| MockNetworkIdentity::new().identity())
            .collect();

        identities.iter().for_each(|identity| {
            test_data
                .network
                .emit_event(Event::NotificationStreamOpened {
                    protocol: Cow::Borrowed(ALEPH_PROTOCOL_NAME),
                    remote: identity.1.into(),
                    negotiated_fallback: None,
                    role: ObservedRole::Authority,
                })
        });

        // We do this only to make sure that NotificationStreamOpened events are handled
        test_data.wait_for_events_handled().await;

        let message: Vec<u8> = vec![1, 2, 3];
        test_data
            .mock_io
            .messages_for_user
            .unbounded_send((message.clone(), DataCommand::Broadcast))
            .unwrap();

        let broadcasted_messages = HashSet::<_>::from_iter(
            test_data
                .network
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

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_notification_stream_closed() {
        let mut test_data = TestData::prepare().await;

        let identities: Vec<_> = (0..4)
            .map(|_| MockNetworkIdentity::new().identity())
            .collect();
        let opened_authorities_n = 2;

        identities.iter().for_each(|identity| {
            test_data
                .network
                .emit_event(Event::NotificationStreamOpened {
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
                test_data
                    .network
                    .emit_event(Event::NotificationStreamClosed {
                        protocol: Cow::Borrowed(ALEPH_PROTOCOL_NAME),
                        remote: identity.1.into(),
                    })
            });

        // We do this only to make sure that NotificationStreamClosed events are handled
        test_data.wait_for_events_handled().await;

        let messages: Vec<Vec<u8>> = vec![vec![1, 2, 3], vec![4, 5, 6]];
        messages.iter().for_each(|m| {
            test_data
                .mock_io
                .messages_for_user
                .unbounded_send((m.clone(), DataCommand::Broadcast))
                .unwrap();
        });

        let broadcasted_messages = HashSet::<_>::from_iter(
            test_data
                .network
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

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_data_command_send_to() {
        let mut test_data = TestData::prepare().await;

        let identity = MockNetworkIdentity::new().identity();

        let message: Vec<u8> = vec![1, 2, 3];

        test_data
            .network
            .emit_event(Event::NotificationStreamOpened {
                protocol: Cow::Borrowed(ALEPH_PROTOCOL_NAME),
                remote: identity.1.into(),
                negotiated_fallback: None,
                role: ObservedRole::Authority,
            });

        // We do this only to make sure that NotificationStreamOpened events are handled
        test_data.wait_for_events_handled().await;

        test_data
            .mock_io
            .messages_for_user
            .unbounded_send((
                message.clone(),
                DataCommand::SendTo(identity.1, Protocol::Validator),
            ))
            .unwrap();

        let expected = (message.encode(), identity.1, Protocol::Validator.name());

        assert_eq!(
            test_data
                .network
                .send_message
                .1
                .lock()
                .next()
                .await
                .expect("Should receive message"),
            expected,
        );

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_create_sender_error_one_peer() {
        let mut test_data = TestData::prepare().await;

        test_data
            .network
            .create_sender_errors
            .lock()
            .push_back(MockSenderError::SomeError);

        let identity = MockNetworkIdentity::new().identity();

        let message_1: Vec<u8> = vec![1, 2, 3];
        let message_2: Vec<u8> = vec![4, 5, 6];

        test_data
            .network
            .emit_event(Event::NotificationStreamOpened {
                protocol: Cow::Borrowed(ALEPH_PROTOCOL_NAME),
                remote: identity.1.into(),
                negotiated_fallback: None,
                role: ObservedRole::Authority,
            });

        // We do this only to make sure that NotificationStreamOpened events are handled
        test_data.wait_for_events_handled().await;

        test_data
            .mock_io
            .messages_for_user
            .unbounded_send((
                message_1.clone(),
                DataCommand::SendTo(identity.1, Protocol::Validator),
            ))
            .unwrap();

        test_data
            .mock_io
            .messages_for_user
            .unbounded_send((
                message_2.clone(),
                DataCommand::SendTo(identity.1, Protocol::Validator),
            ))
            .unwrap();

        let expected = (message_2.encode(), identity.1, Protocol::Validator.name());

        assert_eq!(
            test_data
                .network
                .send_message
                .1
                .lock()
                .next()
                .await
                .expect("Should receive message"),
            expected,
        );

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_create_sender_error_many_peers() {
        let mut test_data = TestData::prepare().await;

        let all_authorities_n = 4;
        let closed_authorities_n = 2;
        for _ in 0..closed_authorities_n {
            test_data
                .network
                .create_sender_errors
                .lock()
                .push_back(MockSenderError::SomeError);
        }

        let identities: Vec<_> = (0..4)
            .map(|_| MockNetworkIdentity::new().identity())
            .collect();
        let message: Vec<u8> = vec![1, 2, 3];

        identities.iter().for_each(|identity| {
            test_data
                .network
                .emit_event(Event::NotificationStreamOpened {
                    protocol: Cow::Borrowed(ALEPH_PROTOCOL_NAME),
                    remote: identity.1.into(),
                    negotiated_fallback: None,
                    role: ObservedRole::Authority,
                })
        });

        // We do this only to make sure that NotificationStreamOpened events are handled
        test_data.wait_for_events_handled().await;

        identities.iter().for_each(|identity| {
            test_data
                .mock_io
                .messages_for_user
                .unbounded_send((
                    message.clone(),
                    DataCommand::SendTo(identity.1, Protocol::Validator),
                ))
                .unwrap();
        });

        let broadcasted_messages = HashSet::<_>::from_iter(
            test_data
                .network
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

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_data_command_send_to_error_one_peer() {
        let mut test_data = TestData::prepare().await;

        test_data
            .network
            .send_errors
            .lock()
            .push_back(MockSenderError::SomeError);

        let identity = MockNetworkIdentity::new().identity();

        let message_1: Vec<u8> = vec![1, 2, 3];
        let message_2: Vec<u8> = vec![4, 5, 6];

        test_data
            .network
            .emit_event(Event::NotificationStreamOpened {
                protocol: Cow::Borrowed(ALEPH_PROTOCOL_NAME),
                remote: identity.1.into(),
                negotiated_fallback: None,
                role: ObservedRole::Authority,
            });

        // We do this only to make sure that NotificationStreamOpened events are handled
        test_data.wait_for_events_handled().await;

        test_data
            .mock_io
            .messages_for_user
            .unbounded_send((
                message_1.clone(),
                DataCommand::SendTo(identity.1, Protocol::Validator),
            ))
            .unwrap();

        test_data
            .mock_io
            .messages_for_user
            .unbounded_send((
                message_2.clone(),
                DataCommand::SendTo(identity.1, Protocol::Validator),
            ))
            .unwrap();

        let expected = (message_2.encode(), identity.1, Protocol::Validator.name());

        assert_eq!(
            test_data
                .network
                .send_message
                .1
                .lock()
                .next()
                .await
                .expect("Should receive message"),
            expected,
        );

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_data_command_send_to_error_many_peers() {
        let mut test_data = TestData::prepare().await;

        let all_authorities_n = 4;
        let closed_authorities_n = 2;
        for _ in 0..closed_authorities_n {
            test_data
                .network
                .send_errors
                .lock()
                .push_back(MockSenderError::SomeError);
        }

        let identities: Vec<_> = (0..4)
            .map(|_| MockNetworkIdentity::new().identity())
            .collect();
        let message: Vec<u8> = vec![1, 2, 3];

        identities.iter().for_each(|identity| {
            test_data
                .network
                .emit_event(Event::NotificationStreamOpened {
                    protocol: Cow::Borrowed(ALEPH_PROTOCOL_NAME),
                    remote: identity.1.into(),
                    negotiated_fallback: None,
                    role: ObservedRole::Authority,
                })
        });

        // We do this only to make sure that NotificationStreamOpened events are handled
        test_data.wait_for_events_handled().await;

        identities.iter().for_each(|identity| {
            test_data
                .mock_io
                .messages_for_user
                .unbounded_send((
                    message.clone(),
                    DataCommand::SendTo(identity.1, Protocol::Validator),
                ))
                .unwrap();
        });

        let broadcasted_messages = HashSet::<_>::from_iter(
            test_data
                .network
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

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_notification_received() {
        let mut test_data = TestData::prepare().await;

        let identity = MockNetworkIdentity::new().identity();

        let message: Vec<u8> = vec![1, 2, 3];
        let incorrect_message: Vec<u8> = vec![4, 5, 6];

        test_data.network.emit_event(Event::NotificationsReceived {
            remote: identity.1.into(),
            messages: vec![(
                Cow::Borrowed("INCORRECT/PROTOCOL/NAME"),
                Vec::encode(&incorrect_message).into(),
            )],
        });

        test_data.network.emit_event(Event::NotificationsReceived {
            remote: identity.1.into(),
            messages: vec![(
                Cow::Borrowed(ALEPH_PROTOCOL_NAME),
                Vec::encode(&message).into(),
            )],
        });

        assert_eq!(
            test_data
                .mock_io
                .messages_from_user
                .next()
                .await
                .expect("Should receive message"),
            message
        );

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_command_add_reserved() {
        let test_data = TestData::prepare().await;

        let identity = MockNetworkIdentity::new().identity();

        test_data
            .mock_io
            .commands_for_manager
            .unbounded_send(ConnectionCommand::AddReserved(
                identity.0.clone().into_iter().collect(),
            ))
            .unwrap();

        let expected = (
            identity.0.into_iter().collect(),
            Cow::Borrowed(ALEPH_VALIDATOR_PROTOCOL_NAME),
        );

        assert_eq!(
            test_data
                .network
                .add_reserved
                .1
                .lock()
                .next()
                .await
                .expect("Should receive message"),
            expected
        );

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_command_remove_reserved() {
        let test_data = TestData::prepare().await;

        let identity = MockNetworkIdentity::new().identity();

        test_data
            .mock_io
            .commands_for_manager
            .unbounded_send(ConnectionCommand::DelReserved(
                iter::once(identity.1).collect(),
            ))
            .unwrap();

        let expected = (
            iter::once(identity.1).collect(),
            Cow::Borrowed(ALEPH_VALIDATOR_PROTOCOL_NAME),
        );

        assert_eq!(
            test_data
                .network
                .remove_reserved
                .1
                .lock()
                .next()
                .await
                .expect("Should receive message"),
            expected
        );

        test_data.cleanup().await
    }
}
