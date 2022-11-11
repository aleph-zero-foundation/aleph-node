use std::{
    collections::{HashMap, HashSet},
    future::Future,
    iter,
};

use aleph_primitives::AuthorityId;
use codec::{Decode, Encode};
use futures::{channel::mpsc, StreamExt};
use log::{debug, error, info, trace, warn};
use sc_service::SpawnTaskHandle;
use sc_utils::mpsc::{tracing_unbounded, TracingUnboundedReceiver, TracingUnboundedSender};
use tokio::time;

use super::manager::DataInSession;
use crate::{
    network::{
        manager::{NetworkData, VersionedAuthentication},
        ConnectionCommand, Data, DataCommand, Event, EventStream, Multiaddress, Network,
        NetworkSender, Protocol,
    },
    validator_network::Network as ValidatorNetwork,
    STATUS_REPORT_INTERVAL,
};

type MessageFromUser<D, A> = (NetworkData<D, A>, DataCommand<<A as Multiaddress>::PeerId>);
/// A service managing all the direct interaction with the underlying network implementation. It
/// handles:
/// 1. Incoming network events
///   1. Messages are forwarded to the user.
///   2. Various forms of (dis)connecting, keeping track of all currently connected nodes.
/// 2. Commands from the network manager, modifying the reserved peer set.
/// 3. Outgoing messages, sending them out, using 1.2. to broadcast.
/// For the time of transition from old validator network (called legacy here) to new tcp validator network
/// we need to support both networks here. To do that we rename legacy network methods to have prefix `legacy_`.
/// We also support two connection managers one for each network.
pub struct Service<
    N: Network,
    D: Data,
    A: Data + Multiaddress<PeerId = AuthorityId>,
    VN: ValidatorNetwork<A, DataInSession<D>>,
> {
    network: N,
    validator_network: VN,
    messages_from_user: mpsc::UnboundedReceiver<MessageFromUser<D, A>>,
    messages_for_user: mpsc::UnboundedSender<NetworkData<D, A>>,
    commands_from_manager: mpsc::UnboundedReceiver<ConnectionCommand<A>>,
    authentication_connected_peers: HashSet<N::PeerId>,
    authentication_peer_senders:
        HashMap<N::PeerId, TracingUnboundedSender<VersionedAuthentication<A>>>,
    spawn_handle: SpawnTaskHandle,
}

/// Input/output channels for the network service.
pub struct IO<D: Data, M: Multiaddress> {
    pub messages_from_user: mpsc::UnboundedReceiver<(D, DataCommand<M::PeerId>)>,
    pub messages_for_user: mpsc::UnboundedSender<D>,
    pub commands_from_manager: mpsc::UnboundedReceiver<ConnectionCommand<M>>,
}

impl<D: Data, M: Multiaddress> IO<D, M> {
    pub fn new(
        messages_from_user: mpsc::UnboundedReceiver<(D, DataCommand<M::PeerId>)>,
        messages_for_user: mpsc::UnboundedSender<D>,
        commands_from_manager: mpsc::UnboundedReceiver<ConnectionCommand<M>>,
    ) -> IO<D, M> {
        IO {
            messages_from_user,
            messages_for_user,
            commands_from_manager,
        }
    }
}

#[derive(Debug)]
enum SendError {
    MissingSender,
    SendingFailed,
}

impl<
        N: Network,
        D: Data,
        A: Data + Multiaddress<PeerId = AuthorityId>,
        VN: ValidatorNetwork<A, DataInSession<D>>,
    > Service<N, D, A, VN>
{
    pub fn new(
        network: N,
        validator_network: VN,
        spawn_handle: SpawnTaskHandle,
        io: IO<NetworkData<D, A>, A>,
    ) -> Service<N, D, A, VN> {
        Service {
            network,
            validator_network,
            messages_from_user: io.messages_from_user,
            messages_for_user: io.messages_for_user,
            commands_from_manager: io.commands_from_manager,
            spawn_handle,
            authentication_connected_peers: HashSet::new(),
            authentication_peer_senders: HashMap::new(),
        }
    }

    fn get_sender(
        &mut self,
        peer: &N::PeerId,
        protocol: Protocol,
    ) -> Option<&mut TracingUnboundedSender<VersionedAuthentication<A>>> {
        match protocol {
            Protocol::Authentication => self.authentication_peer_senders.get_mut(peer),
        }
    }

    fn peer_sender(
        &self,
        peer_id: N::PeerId,
        mut receiver: TracingUnboundedReceiver<VersionedAuthentication<A>>,
        protocol: Protocol,
    ) -> impl Future<Output = ()> + Send + 'static {
        let network = self.network.clone();
        async move {
            let mut sender = None;
            loop {
                if let Some(data) = receiver.next().await {
                    let s = if let Some(s) = sender.as_mut() {
                        s
                    } else {
                        match network.sender(peer_id.clone(), protocol) {
                            Ok(s) => sender.insert(s),
                            Err(e) => {
                                debug!(target: "aleph-network", "Failed creating sender. Dropping message: {}", e);
                                continue;
                            }
                        }
                    };
                    if let Err(e) = s.send(data.encode()).await {
                        debug!(target: "aleph-network", "Failed sending data to peer. Dropping sender and message: {}", e);
                        sender = None;
                    }
                } else {
                    debug!(target: "aleph-network", "Sender was dropped for peer {:?}. Peer sender exiting.", peer_id);
                    return;
                }
            }
        }
    }

    fn send_to_peer(
        &mut self,
        data: VersionedAuthentication<A>,
        peer: N::PeerId,
        protocol: Protocol,
    ) -> Result<(), SendError> {
        match self.get_sender(&peer, protocol) {
            Some(sender) => {
                match sender.unbounded_send(data) {
                    Err(e) => {
                        // Receiver can also be dropped when thread cannot send to peer. In case receiver is dropped this entry will be removed by Event::NotificationStreamClosed
                        // No need to remove the entry here
                        if e.is_disconnected() {
                            trace!(target: "aleph-network", "Failed sending data to peer because peer_sender receiver is dropped: {:?}", peer);
                        }
                        Err(SendError::SendingFailed)
                    }
                    Ok(_) => Ok(()),
                }
            }
            None => Err(SendError::MissingSender),
        }
    }

    fn broadcast(&mut self, data: VersionedAuthentication<A>, protocol: Protocol) {
        let peers = match protocol {
            // Validator protocol will never broadcast.
            Protocol::Authentication => self.authentication_connected_peers.clone(),
        };
        for peer in peers {
            // We only broadcast authentication information in this sense, so we use the generic
            // Protocol.
            if let Err(e) = self.send_to_peer(data.clone(), peer.clone(), protocol) {
                trace!(target: "aleph-network", "Failed to send broadcast to peer{:?}, {:?}", peer, e);
            }
        }
    }

    fn handle_network_event(
        &mut self,
        event: Event<N::Multiaddress, N::PeerId>,
    ) -> Result<(), mpsc::TrySendError<NetworkData<D, A>>> {
        use Event::*;
        match event {
            Connected(multiaddress) => {
                trace!(target: "aleph-network", "Connected event from address {:?}", multiaddress);
                self.network
                    .add_reserved(iter::once(multiaddress).collect(), Protocol::Authentication);
            }
            Disconnected(peer) => {
                trace!(target: "aleph-network", "Disconnected event for peer {:?}", peer);
                self.network
                    .remove_reserved(iter::once(peer).collect(), Protocol::Authentication);
            }
            StreamOpened(peer, protocol) => {
                trace!(target: "aleph-network", "StreamOpened event for peer {:?} and the protocol {:?}.", peer, protocol);
                let rx = match &protocol {
                    Protocol::Authentication => {
                        let (tx, rx) = tracing_unbounded("mpsc_notification_stream_authentication");
                        self.authentication_connected_peers.insert(peer.clone());
                        self.authentication_peer_senders.insert(peer.clone(), tx);
                        rx
                    }
                };
                self.spawn_handle.spawn(
                    "aleph/network/peer_sender",
                    None,
                    self.peer_sender(peer, rx, protocol),
                );
            }
            StreamClosed(peer, protocol) => {
                trace!(target: "aleph-network", "StreamClosed event for peer {:?} and protocol {:?}", peer, protocol);
                match protocol {
                    Protocol::Authentication => {
                        self.authentication_connected_peers.remove(&peer);
                        self.authentication_peer_senders.remove(&peer);
                    }
                }
            }
            Messages(messages) => {
                for (protocol, data) in messages.into_iter() {
                    match protocol {
                        Protocol::Authentication => {
                            match VersionedAuthentication::<A>::decode(&mut &data[..])
                                .map(|a| a.try_into())
                            {
                                Ok(Ok(data)) => self.messages_for_user.unbounded_send(data)?,
                                Ok(Err(e)) => {
                                    warn!(target: "aleph-network", "Error decoding authentication protocol message: {}", e)
                                }
                                Err(e) => {
                                    warn!(target: "aleph-network", "Error decoding authentication protocol message: {}", e)
                                }
                            }
                        }
                    };
                }
            }
        }
        Ok(())
    }

    fn handle_validator_network_data(
        &mut self,
        data: DataInSession<D>,
    ) -> Result<(), mpsc::TrySendError<NetworkData<D, A>>> {
        self.messages_for_user.unbounded_send(data.into())
    }

    fn on_manager_command(&mut self, command: ConnectionCommand<A>) {
        use ConnectionCommand::*;
        match command {
            AddReserved(addresses) => {
                for multi in addresses {
                    if let Some(peer_id) = multi.get_peer_id() {
                        self.validator_network.add_connection(peer_id, vec![multi]);
                    }
                }
            }
            DelReserved(peers) => {
                for peer in peers {
                    self.validator_network.remove_connection(peer);
                }
            }
        }
    }

    fn on_user_message(&mut self, data: NetworkData<D, A>, command: DataCommand<A::PeerId>) {
        use DataCommand::*;

        match data {
            NetworkData::Meta(discovery_message) => {
                let data: VersionedAuthentication<A> = discovery_message.into();
                match command {
                    Broadcast => self.broadcast(data, Protocol::Authentication),
                    SendTo(_) => {
                        // We ignore this for now. Sending Meta messages to peer is an optimization done for the sake of tests.
                    }
                }
            }
            NetworkData::Data(data, session_id) => {
                match command {
                    Broadcast => {
                        // We ignore this for now. AlephBFT does not broadcast data.
                    }
                    SendTo(peer) => self
                        .validator_network
                        .send(DataInSession { data, session_id }, peer),
                }
            }
        }
    }

    fn status_report(&self) {
        let mut status = String::from("Network status report: ");

        status.push_str(&format!(
            "authentication connected peers - {:?}; ",
            self.authentication_connected_peers.len()
        ));

        info!(target: "aleph-network", "{}", status);
    }

    pub async fn run(mut self) {
        let mut events_from_network = self.network.event_stream();

        let mut status_ticker = time::interval(STATUS_REPORT_INTERVAL);
        loop {
            tokio::select! {
                maybe_event = events_from_network.next_event() => match maybe_event {
                    Some(event) => if let Err(e) = self.handle_network_event(event) {
                        error!(target: "aleph-network", "Cannot forward messages to user: {:?}", e);
                        return;
                    },
                    None => {
                        error!(target: "aleph-network", "Network event stream ended.");
                        return;
                    }
                },
                maybe_data = self.validator_network.next() => match maybe_data {
                    Some(data) => if let Err(e) = self.handle_validator_network_data(data) {
                        error!(target: "aleph-network", "Cannot forward messages to user: {:?}", e);
                        return;
                    },
                    None => {
                        error!(target: "aleph-network", "Validator network event stream ended.");
                    }
                },
                maybe_message = self.messages_from_user.next() => match maybe_message {
                    Some((data, command)) => self.on_user_message(data, command),
                    None => {
                        error!(target: "aleph-network", "User message stream ended.");
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
                _ = status_ticker.tick() => {
                    self.status_report();
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, iter, iter::FromIterator};

    use codec::Encode;
    use futures::{channel::oneshot, StreamExt};
    use sc_service::TaskManager;
    use tokio::{runtime::Handle, task::JoinHandle};

    use super::{ConnectionCommand, DataCommand, Service};
    use crate::{
        network::{
            manager::{DataInSession, SessionHandler, VersionedAuthentication},
            mock::{
                crypto_basics, MockData, MockEvent, MockIO,
                MockMultiaddress as MockAuthMultiaddress, MockNetwork,
                MockPeerId as MockAuthPeerId, MockSenderError,
            },
            testing::{DiscoveryMessage, NetworkData},
            NetworkIdentity, Protocol,
        },
        // session::SessionId,
        testing::mocks::validator_network::{
            random_authority_id, MockMultiaddress, MockNetwork as MockValidatorNetwork,
        },
        SessionId,
    };

    pub struct TestData {
        pub service_handle: JoinHandle<()>,
        pub exit_tx: oneshot::Sender<()>,
        pub network: MockNetwork,
        pub validator_network: MockValidatorNetwork<DataInSession<MockData>>,
        pub mock_io: MockIO<MockMultiaddress>,
        // `TaskManager` can't be dropped for `SpawnTaskHandle` to work
        _task_manager: TaskManager,
    }

    impl TestData {
        async fn prepare() -> Self {
            let task_manager = TaskManager::new(Handle::current(), None).unwrap();

            // Prepare communication with service
            let (mock_io, io) = MockIO::new();
            // Prepare service
            let (event_stream_oneshot_tx, event_stream_oneshot_rx) = oneshot::channel();
            let network = MockNetwork::new(event_stream_oneshot_tx);
            let validator_network = MockValidatorNetwork::new("addr").await;
            let service = Service::new(
                network.clone(),
                validator_network.clone(),
                task_manager.spawn_handle(),
                io,
            );
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
                validator_network,
                mock_io,
                _task_manager: task_manager,
            }
        }

        async fn cleanup(self) {
            self.exit_tx.send(()).unwrap();
            self.service_handle.await.unwrap();
            self.network.close_channels().await;
            self.validator_network.close_channels().await;
        }

        // We do this only to make sure that NotificationStreamOpened/NotificationStreamClosed events are handled
        async fn wait_for_events_handled(&mut self) {
            let address = MockAuthMultiaddress::random_with_id(MockAuthPeerId::random());
            self.network
                .emit_event(MockEvent::Connected(address.clone()));
            assert_eq!(
                self.network
                    .add_reserved
                    .next()
                    .await
                    .expect("Should receive message"),
                (iter::once(address).collect(), Protocol::Authentication,)
            );
        }
    }

    fn message(i: u8) -> DataInSession<MockData> {
        DataInSession {
            data: vec![i, i + 1, i + 2],
            session_id: SessionId(1),
        }
    }

    async fn authentication(
        multiaddresses: Vec<MockMultiaddress>,
    ) -> DiscoveryMessage<MockMultiaddress> {
        let crypto_basics = crypto_basics(1).await;
        let handler = SessionHandler::new(
            Some(crypto_basics.0[0].clone()),
            crypto_basics.1.clone(),
            SessionId(43),
            multiaddresses,
        )
        .await
        .unwrap();
        DiscoveryMessage::AuthenticationBroadcast(handler.authentication().unwrap())
    }

    #[tokio::test]
    async fn test_sync_connected() {
        let mut test_data = TestData::prepare().await;

        let address = MockAuthMultiaddress::random_with_id(MockAuthPeerId::random());
        test_data
            .network
            .emit_event(MockEvent::Connected(address.clone()));

        let expected = (iter::once(address).collect(), Protocol::Authentication);

        assert_eq!(
            test_data
                .network
                .add_reserved
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

        let peer_id = MockAuthPeerId::random();

        test_data
            .network
            .emit_event(MockEvent::Disconnected(peer_id));

        let expected = (iter::once(peer_id).collect(), Protocol::Authentication);

        assert_eq!(
            test_data
                .network
                .remove_reserved
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

        let peer_ids: Vec<_> = (0..3).map(|_| MockAuthPeerId::random()).collect();

        peer_ids.iter().for_each(|peer_id| {
            test_data
                .network
                .emit_event(MockEvent::StreamOpened(*peer_id, Protocol::Authentication));
        });

        // We do this only to make sure that NotificationStreamOpened events are handled
        test_data.wait_for_events_handled().await;

        let message = authentication(test_data.validator_network.identity().0).await;
        test_data
            .mock_io
            .messages_for_user
            .unbounded_send((NetworkData::Meta(message.clone()), DataCommand::Broadcast))
            .unwrap();

        let broadcasted_messages = HashSet::<_>::from_iter(
            test_data
                .network
                .send_message
                .take(peer_ids.len())
                .await
                .into_iter(),
        );

        let expected_messages = HashSet::from_iter(peer_ids.into_iter().map(|peer_id| {
            (
                VersionedAuthentication::V1(message.clone()).encode(),
                peer_id,
                Protocol::Authentication,
            )
        }));

        assert_eq!(broadcasted_messages, expected_messages);

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_notification_stream_closed() {
        let mut test_data = TestData::prepare().await;

        let peer_ids: Vec<_> = (0..3).map(|_| MockAuthPeerId::random()).collect();
        let opened_authorities_n = 2;

        peer_ids.iter().for_each(|peer_id| {
            test_data
                .network
                .emit_event(MockEvent::StreamOpened(*peer_id, Protocol::Authentication));
        });

        peer_ids
            .iter()
            .skip(opened_authorities_n)
            .for_each(|peer_id| {
                test_data
                    .network
                    .emit_event(MockEvent::StreamClosed(*peer_id, Protocol::Authentication));
            });

        // We do this only to make sure that NotificationStreamClosed events are handled
        test_data.wait_for_events_handled().await;

        let message = authentication(test_data.validator_network.identity().0).await;
        test_data
            .mock_io
            .messages_for_user
            .unbounded_send((NetworkData::Meta(message.clone()), DataCommand::Broadcast))
            .unwrap();

        let broadcasted_messages = HashSet::<_>::from_iter(
            test_data
                .network
                .send_message
                .take(opened_authorities_n)
                .await
                .into_iter(),
        );

        let expected_messages = HashSet::from_iter(
            peer_ids
                .into_iter()
                .take(opened_authorities_n)
                .map(|peer_id| {
                    (
                        VersionedAuthentication::V1(message.clone()).encode(),
                        peer_id,
                        Protocol::Authentication,
                    )
                }),
        );

        assert_eq!(broadcasted_messages, expected_messages);

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_validator_data_command_send_to() {
        let mut test_data = TestData::prepare().await;

        let peer_id = random_authority_id().await;

        let message = message(1);

        test_data
            .mock_io
            .messages_for_user
            .unbounded_send((message.clone().into(), DataCommand::SendTo(peer_id.clone())))
            .unwrap();

        let expected = (message, peer_id);

        assert_eq!(
            test_data
                .validator_network
                .send
                .next()
                .await
                .expect("Should receive message"),
            expected,
        );

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_receives_validator_data() {
        let mut test_data = TestData::prepare().await;

        let message = message(1);

        test_data.validator_network.next.send(message.clone());

        let expected: NetworkData<_, _> = message.into();

        assert_eq!(
            test_data
                .mock_io
                .messages_from_user
                .next()
                .await
                .expect("Should receive message"),
            expected,
        );

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_create_sender_error() {
        let mut test_data = TestData::prepare().await;

        test_data
            .network
            .create_sender_errors
            .lock()
            .push_back(MockSenderError::SomeError);

        let peer_id = MockAuthPeerId::random();

        let message_1 =
            authentication(vec![(random_authority_id().await, String::from("other_1"))]).await;
        let message_2 =
            authentication(vec![(random_authority_id().await, String::from("other_2"))]).await;

        test_data
            .network
            .emit_event(MockEvent::StreamOpened(peer_id, Protocol::Authentication));

        // We do this only to make sure that NotificationStreamOpened events are handled
        test_data.wait_for_events_handled().await;

        test_data
            .mock_io
            .messages_for_user
            .unbounded_send((NetworkData::Meta(message_1), DataCommand::Broadcast))
            .unwrap();

        test_data
            .mock_io
            .messages_for_user
            .unbounded_send((NetworkData::Meta(message_2.clone()), DataCommand::Broadcast))
            .unwrap();

        let expected = (
            VersionedAuthentication::V1(message_2).encode(),
            peer_id,
            Protocol::Authentication,
        );

        assert_eq!(
            test_data
                .network
                .send_message
                .next()
                .await
                .expect("Should receive message"),
            expected,
        );

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_send_error() {
        let mut test_data = TestData::prepare().await;

        test_data
            .network
            .send_errors
            .lock()
            .push_back(MockSenderError::SomeError);

        let peer_id = MockAuthPeerId::random();

        let message_1 =
            authentication(vec![(random_authority_id().await, String::from("other_1"))]).await;
        let message_2 =
            authentication(vec![(random_authority_id().await, String::from("other_2"))]).await;

        test_data
            .network
            .emit_event(MockEvent::StreamOpened(peer_id, Protocol::Authentication));

        // We do this only to make sure that NotificationStreamOpened events are handled
        test_data.wait_for_events_handled().await;

        test_data
            .mock_io
            .messages_for_user
            .unbounded_send((NetworkData::Meta(message_1), DataCommand::Broadcast))
            .unwrap();

        test_data
            .mock_io
            .messages_for_user
            .unbounded_send((NetworkData::Meta(message_2.clone()), DataCommand::Broadcast))
            .unwrap();

        let expected = (
            VersionedAuthentication::V1(message_2).encode(),
            peer_id,
            Protocol::Authentication,
        );

        assert_eq!(
            test_data
                .network
                .send_message
                .next()
                .await
                .expect("Should receive message"),
            expected,
        );

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_notification_received() {
        let mut test_data = TestData::prepare().await;

        let message = authentication(vec![(
            random_authority_id().await,
            String::from("other_addr"),
        )])
        .await;

        test_data.network.emit_event(MockEvent::Messages(vec![(
            Protocol::Authentication,
            VersionedAuthentication::V1(message.clone()).encode().into(),
        )]));

        let expected = NetworkData::Meta(message);

        assert_eq!(
            test_data
                .mock_io
                .messages_from_user
                .next()
                .await
                .expect("Should receive message"),
            expected
        );

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_command_add_reserved() {
        let mut test_data = TestData::prepare().await;

        let multiaddress: MockMultiaddress =
            (random_authority_id().await, String::from("other_addr"));

        test_data
            .mock_io
            .commands_for_manager
            .unbounded_send(ConnectionCommand::AddReserved(
                iter::once(multiaddress.clone()).collect(),
            ))
            .unwrap();

        let expected = (multiaddress.0.clone(), vec![multiaddress]);

        assert_eq!(
            test_data
                .validator_network
                .add_connection
                .next()
                .await
                .expect("Should receive message"),
            expected
        );

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_command_remove_reserved() {
        let mut test_data = TestData::prepare().await;

        let peer_id = random_authority_id().await;

        test_data
            .mock_io
            .commands_for_manager
            .unbounded_send(ConnectionCommand::DelReserved(
                iter::once(peer_id.clone()).collect(),
            ))
            .unwrap();

        assert_eq!(
            test_data
                .validator_network
                .remove_connection
                .next()
                .await
                .expect("Should receive message"),
            peer_id
        );

        test_data.cleanup().await
    }
}
