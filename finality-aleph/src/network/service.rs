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
    LD: Data,
    A: Data + Multiaddress<PeerId = AuthorityId>,
    VN: ValidatorNetwork<A, DataInSession<D>>,
> {
    network: N,
    validator_network: VN,
    messages_from_user: mpsc::UnboundedReceiver<MessageFromUser<D, A>>,
    messages_for_user: mpsc::UnboundedSender<NetworkData<D, A>>,
    commands_from_manager: mpsc::UnboundedReceiver<ConnectionCommand<A>>,
    // In future these legacy senders and receiver will be removed
    legacy_messages_from_user: mpsc::UnboundedReceiver<(LD, DataCommand<N::PeerId>)>,
    legacy_messages_for_user: mpsc::UnboundedSender<LD>,
    legacy_commands_from_manager: mpsc::UnboundedReceiver<ConnectionCommand<N::Multiaddress>>,
    legacy_generic_connected_peers: HashSet<N::PeerId>,
    legacy_validator_connected_peers: HashSet<N::PeerId>,
    authentication_connected_peers: HashSet<N::PeerId>,
    // For now we need to use `Vec<u8>` here.
    // This is needed for backward compatibility with old network.
    // This can be changed back to `Data` once Legacy Network is removed.
    // In future this will be changed to somethig like `AuthenticationData<A>`.
    legacy_generic_peer_senders: HashMap<N::PeerId, TracingUnboundedSender<Vec<u8>>>,
    legacy_validator_peer_senders: HashMap<N::PeerId, TracingUnboundedSender<Vec<u8>>>,
    authentication_peer_senders: HashMap<N::PeerId, TracingUnboundedSender<Vec<u8>>>,
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

#[derive(Debug)]
enum SendToUserError {
    LegacySender,
    LatestSender,
}

impl<
        N: Network,
        D: Data,
        LD: Data,
        A: Data + Multiaddress<PeerId = AuthorityId>,
        VN: ValidatorNetwork<A, DataInSession<D>>,
    > Service<N, D, LD, A, VN>
{
    pub fn new(
        network: N,
        validator_network: VN,
        spawn_handle: SpawnTaskHandle,
        io: IO<NetworkData<D, A>, A>,
        legacy_io: IO<LD, N::Multiaddress>,
    ) -> Service<N, D, LD, A, VN> {
        Service {
            network,
            validator_network,
            messages_from_user: io.messages_from_user,
            messages_for_user: io.messages_for_user,
            commands_from_manager: io.commands_from_manager,
            legacy_messages_from_user: legacy_io.messages_from_user,
            legacy_messages_for_user: legacy_io.messages_for_user,
            legacy_commands_from_manager: legacy_io.commands_from_manager,
            spawn_handle,
            legacy_generic_connected_peers: HashSet::new(),
            legacy_validator_connected_peers: HashSet::new(),
            authentication_connected_peers: HashSet::new(),
            legacy_generic_peer_senders: HashMap::new(),
            legacy_validator_peer_senders: HashMap::new(),
            authentication_peer_senders: HashMap::new(),
        }
    }

    fn get_sender(
        &mut self,
        peer: &N::PeerId,
        protocol: Protocol,
    ) -> Option<&mut TracingUnboundedSender<Vec<u8>>> {
        match protocol {
            Protocol::Generic => self.legacy_generic_peer_senders.get_mut(peer),
            Protocol::Validator => self.legacy_validator_peer_senders.get_mut(peer),
            Protocol::Authentication => self.authentication_peer_senders.get_mut(peer),
        }
    }

    fn peer_sender(
        &self,
        peer_id: N::PeerId,
        mut receiver: TracingUnboundedReceiver<Vec<u8>>,
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
                    if let Err(e) = s.send(data).await {
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
        data: Vec<u8>,
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

    fn broadcast(&mut self, data: Vec<u8>, protocol: Protocol) {
        let peers = match protocol {
            // Validator protocol will never broadcast.
            Protocol::Validator => HashSet::new(),
            Protocol::Generic => self.legacy_generic_connected_peers.clone(),
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
        event: Event<N::Multiaddress>,
    ) -> Result<(), SendToUserError> {
        use Event::*;
        match event {
            Connected(multiaddress) => {
                trace!(target: "aleph-network", "Connected event from address {:?}", multiaddress);
                self.network.add_reserved(
                    iter::once(multiaddress.clone()).collect(),
                    Protocol::Generic,
                );
                self.network
                    .add_reserved(iter::once(multiaddress).collect(), Protocol::Authentication);
            }
            Disconnected(peer) => {
                trace!(target: "aleph-network", "Disconnected event for peer {:?}", peer);
                self.network
                    .remove_reserved(iter::once(peer.clone()).collect(), Protocol::Generic);
                self.network
                    .remove_reserved(iter::once(peer).collect(), Protocol::Authentication);
            }
            StreamOpened(peer, protocol) => {
                trace!(target: "aleph-network", "StreamOpened event for peer {:?} and the protocol {:?}.", peer, protocol);
                let rx = match &protocol {
                    Protocol::Generic => {
                        let (tx, rx) = tracing_unbounded("mpsc_notification_stream_legacy_generic");
                        self.legacy_generic_connected_peers.insert(peer.clone());
                        self.legacy_generic_peer_senders.insert(peer.clone(), tx);
                        rx
                    }
                    Protocol::Validator => {
                        let (tx, rx) =
                            tracing_unbounded("mpsc_notification_stream_legacy_validator");
                        self.legacy_validator_connected_peers.insert(peer.clone());
                        self.legacy_validator_peer_senders.insert(peer.clone(), tx);
                        rx
                    }
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
                    Protocol::Generic => {
                        self.legacy_generic_connected_peers.remove(&peer);
                        self.legacy_generic_peer_senders.remove(&peer);
                    }
                    Protocol::Validator => {
                        self.legacy_validator_connected_peers.remove(&peer);
                        self.legacy_validator_peer_senders.remove(&peer);
                    }
                    Protocol::Authentication => {
                        self.authentication_connected_peers.remove(&peer);
                        self.authentication_peer_senders.remove(&peer);
                    }
                }
            }
            Messages(messages) => {
                for (protocol, data) in messages.into_iter() {
                    match protocol {
                        Protocol::Generic => match LD::decode(&mut &data[..]) {
                            Ok(data) => self
                                .legacy_messages_for_user
                                .unbounded_send(data)
                                .map_err(|_| SendToUserError::LegacySender)?,
                            Err(e) => {
                                warn!(target: "aleph-network", "Error decoding legacy generic protocol message: {}", e)
                            }
                        },
                        Protocol::Validator => match LD::decode(&mut &data[..]) {
                            Ok(data) => self
                                .legacy_messages_for_user
                                .unbounded_send(data)
                                .map_err(|_| SendToUserError::LegacySender)?,
                            Err(e) => {
                                warn!(target: "aleph-network", "Error decoding legacy validator protocol message: {}", e)
                            }
                        },
                        Protocol::Authentication => {
                            match VersionedAuthentication::<A>::decode(&mut &data[..])
                                .map(|a| a.try_into())
                            {
                                Ok(Ok(data)) => self
                                    .messages_for_user
                                    .unbounded_send(data)
                                    .map_err(|_| SendToUserError::LatestSender)?,
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

    /// This will be removed in the future
    fn legacy_on_manager_command(&mut self, command: ConnectionCommand<N::Multiaddress>) {
        use ConnectionCommand::*;
        match command {
            AddReserved(addresses) => {
                self.network.add_reserved(addresses, Protocol::Validator);
            }
            DelReserved(peers) => self.network.remove_reserved(peers, Protocol::Validator),
        }
    }

    fn on_user_message(&mut self, data: NetworkData<D, A>, command: DataCommand<A::PeerId>) {
        use DataCommand::*;

        match data {
            NetworkData::Meta(discovery_message) => {
                let data: VersionedAuthentication<A> = discovery_message.into();
                match command {
                    Broadcast => self.broadcast(data.encode(), Protocol::Authentication),
                    SendTo(_, _) => {
                        // We ignore this for now. Sending Meta messages to peer is an optimization done for the sake of tests.
                    }
                }
            }
            NetworkData::Data(data, session) => {
                match command {
                    Broadcast => {
                        // We ignore this for now. AlephBFT does not broadcast data.
                    }
                    SendTo(peer, _) => self.validator_network.send((data, session), peer),
                }
            }
        }
    }

    /// This will be removed in the future
    fn legacy_on_user_message(&mut self, data: LD, command: DataCommand<N::PeerId>) {
        use DataCommand::*;
        match command {
            Broadcast => self.broadcast(data.encode(), Protocol::Generic),
            SendTo(peer, protocol) => {
                if let Err(e) = self.send_to_peer(data.encode(), peer.clone(), protocol) {
                    trace!(target: "aleph-network", "Failed to send data to peer{:?} via protocol {:?}, {:?}", peer, protocol, e);
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

        status.push_str(&format!(
            "generic connected peers - {:?}; ",
            self.legacy_generic_connected_peers.len()
        ));

        let peer_ids = self
            .legacy_validator_connected_peers
            .iter()
            .map(|peer_id| format!("{}", peer_id))
            .collect::<Vec<_>>()
            .join(", ");
        status.push_str(&format!(
            "validator connected peers - {:?} [{}]; ",
            self.legacy_validator_connected_peers.len(),
            peer_ids,
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
                        match e {
                            SendToUserError::LegacySender => error!(target: "aleph-network", "Cannot forward messages to user through legacy sender: {:?}", e),
                            SendToUserError::LatestSender => error!(target: "aleph-network", "Cannot forward messages to user: {:?}", e),
                        };
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
                maybe_message = self.legacy_messages_from_user.next() => match maybe_message {
                    Some((data, command)) => self.legacy_on_user_message(data, command),
                    None => {
                        error!(target: "aleph-network", "Legacy user message stream ended.");
                        return;
                    }
                },
                maybe_command = self.legacy_commands_from_manager.next() => match maybe_command {
                    Some(command) => self.legacy_on_manager_command(command),
                    None => {
                        error!(target: "aleph-network", "Legacy manager command stream ended.");
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
            manager::DataInSession,
            mock::{
                MockData, MockEvent, MockIO, MockMultiaddress as LegacyMockMultiaddress,
                MockNetwork, MockNetworkIdentity, MockPeerId, MockSenderError,
            },
            testing::NetworkData,
            NetworkIdentity, Protocol,
        },
        session::SessionId,
        testing::mocks::validator_network::{
            MockMultiaddress, MockNetwork as MockValidatorNetwork,
        },
    };

    pub struct TestData {
        pub service_handle: JoinHandle<()>,
        pub exit_tx: oneshot::Sender<()>,
        pub network: MockNetwork,
        pub validator_network: MockValidatorNetwork<DataInSession<MockData>>,
        pub mock_io: MockIO<MockMultiaddress, LegacyMockMultiaddress>,
        // `TaskManager` can't be dropped for `SpawnTaskHandle` to work
        _task_manager: TaskManager,
    }

    impl TestData {
        async fn prepare() -> Self {
            let task_manager = TaskManager::new(Handle::current(), None).unwrap();

            // Prepare communication with service
            let (mock_io, io, legacy_io) = MockIO::new();
            // Prepare service
            let (event_stream_oneshot_tx, event_stream_oneshot_rx) = oneshot::channel();
            let network = MockNetwork::new(event_stream_oneshot_tx);
            let validator_network = MockValidatorNetwork::new("addr").await;
            let service = Service::new(
                network.clone(),
                validator_network.clone(),
                task_manager.spawn_handle(),
                io,
                legacy_io,
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
        }

        // We do this only to make sure that NotificationStreamOpened/NotificationStreamClosed events are handled
        async fn wait_for_events_handled(&mut self) {
            let address = LegacyMockMultiaddress::random_with_id(MockPeerId::random());
            self.network
                .emit_event(MockEvent::Connected(address.clone()));
            assert_eq!(
                self.network
                    .add_reserved
                    .next()
                    .await
                    .expect("Should receive message"),
                (iter::once(address).collect(), Protocol::Generic,)
            );
        }
    }

    fn message(i: u8) -> NetworkData<MockData, LegacyMockMultiaddress> {
        NetworkData::Data(vec![i, i + 1, i + 2], SessionId(1))
    }

    #[tokio::test]
    async fn test_sync_connected() {
        let mut test_data = TestData::prepare().await;

        let address = LegacyMockMultiaddress::random_with_id(MockPeerId::random());
        test_data
            .network
            .emit_event(MockEvent::Connected(address.clone()));

        let expected = (iter::once(address).collect(), Protocol::Generic);

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

        let peer_id = MockPeerId::random();

        test_data
            .network
            .emit_event(MockEvent::Disconnected(peer_id));

        let expected = (iter::once(peer_id).collect(), Protocol::Generic);

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

        let peer_ids: Vec<_> = (0..3).map(|_| MockPeerId::random()).collect();

        peer_ids.iter().for_each(|peer_id| {
            test_data
                .network
                .emit_event(MockEvent::StreamOpened(*peer_id, Protocol::Generic));
        });

        // We do this only to make sure that NotificationStreamOpened events are handled
        test_data.wait_for_events_handled().await;

        let message = message(1);
        test_data
            .mock_io
            .legacy_messages_for_user
            .unbounded_send((message.clone(), DataCommand::Broadcast))
            .unwrap();

        let broadcasted_messages = HashSet::<_>::from_iter(
            test_data
                .network
                .send_message
                .1
                .lock()
                .await
                .by_ref()
                .take(peer_ids.len())
                .collect::<Vec<_>>()
                .await
                .into_iter(),
        );

        let expected_messages = HashSet::from_iter(
            peer_ids
                .into_iter()
                .map(|peer_id| (message.encode(), peer_id, Protocol::Generic)),
        );

        assert_eq!(broadcasted_messages, expected_messages);

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_notification_stream_closed() {
        let mut test_data = TestData::prepare().await;

        let peer_ids: Vec<_> = (0..4).map(|_| MockPeerId::random()).collect();
        let opened_authorities_n = 2;

        peer_ids.iter().for_each(|peer_id| {
            test_data
                .network
                .emit_event(MockEvent::StreamOpened(*peer_id, Protocol::Generic));
        });

        peer_ids
            .iter()
            .skip(opened_authorities_n)
            .for_each(|peer_id| {
                test_data
                    .network
                    .emit_event(MockEvent::StreamClosed(*peer_id, Protocol::Generic));
            });

        // We do this only to make sure that NotificationStreamClosed events are handled
        test_data.wait_for_events_handled().await;

        let messages: Vec<_> = vec![message(1), message(2)];
        messages.iter().for_each(|m| {
            test_data
                .mock_io
                .legacy_messages_for_user
                .unbounded_send((m.clone(), DataCommand::Broadcast))
                .unwrap();
        });

        let broadcasted_messages = HashSet::<_>::from_iter(
            test_data
                .network
                .send_message
                .1
                .lock()
                .await
                .by_ref()
                .take(opened_authorities_n * messages.len())
                .collect::<Vec<_>>()
                .await
                .into_iter(),
        );

        let expected_messages =
            HashSet::from_iter(peer_ids.into_iter().take(opened_authorities_n).flat_map(
                |peer_id| {
                    messages
                        .iter()
                        .map(move |m| (m.encode(), peer_id, Protocol::Generic))
                },
            ));

        assert_eq!(broadcasted_messages, expected_messages);

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_validator_data_command_send_to() {
        let mut test_data = TestData::prepare().await;

        let peer_id = MockPeerId::random();

        let message = message(1);

        test_data
            .network
            .emit_event(MockEvent::StreamOpened(peer_id, Protocol::Validator));

        // We do this only to make sure that NotificationStreamOpened events are handled
        test_data.wait_for_events_handled().await;

        test_data
            .mock_io
            .legacy_messages_for_user
            .unbounded_send((
                message.clone(),
                DataCommand::SendTo(peer_id, Protocol::Validator),
            ))
            .unwrap();

        let expected = (message.encode(), peer_id, Protocol::Validator);

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
    async fn test_validator_create_sender_error_one_peer() {
        let mut test_data = TestData::prepare().await;

        test_data
            .network
            .create_sender_errors
            .lock()
            .push_back(MockSenderError::SomeError);

        let peer_id = MockPeerId::random();

        let message_1 = message(1);
        let message_2 = message(2);

        test_data
            .network
            .emit_event(MockEvent::StreamOpened(peer_id, Protocol::Validator));

        // We do this only to make sure that NotificationStreamOpened events are handled
        test_data.wait_for_events_handled().await;

        test_data
            .mock_io
            .legacy_messages_for_user
            .unbounded_send((
                message_1.clone(),
                DataCommand::SendTo(peer_id, Protocol::Validator),
            ))
            .unwrap();

        test_data
            .mock_io
            .legacy_messages_for_user
            .unbounded_send((
                message_2.clone(),
                DataCommand::SendTo(peer_id, Protocol::Validator),
            ))
            .unwrap();

        let expected = (message_2.encode(), peer_id, Protocol::Validator);

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
    async fn test_validator_create_sender_error_many_peers() {
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

        let peer_ids: Vec<_> = (0..4).map(|_| MockPeerId::random()).collect();
        let message = message(1);

        peer_ids.iter().for_each(|peer_id| {
            test_data
                .network
                .emit_event(MockEvent::StreamOpened(*peer_id, Protocol::Validator));
        });

        // We do this only to make sure that NotificationStreamOpened events are handled
        test_data.wait_for_events_handled().await;

        peer_ids.iter().for_each(|peer_id| {
            test_data
                .mock_io
                .legacy_messages_for_user
                .unbounded_send((
                    message.clone(),
                    DataCommand::SendTo(*peer_id, Protocol::Validator),
                ))
                .unwrap();
        });

        let broadcasted_messages = HashSet::<_>::from_iter(
            test_data
                .network
                .send_message
                .1
                .lock()
                .await
                .by_ref()
                .take(all_authorities_n - closed_authorities_n)
                .collect::<Vec<_>>()
                .await
                .into_iter(),
        );

        let expected_messages = HashSet::from_iter(
            peer_ids
                .into_iter()
                .skip(closed_authorities_n)
                .map(|peer_id| (message.encode(), peer_id, Protocol::Validator)),
        );

        assert_eq!(broadcasted_messages, expected_messages);

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_validator_data_command_send_to_error_one_peer() {
        let mut test_data = TestData::prepare().await;

        test_data
            .network
            .send_errors
            .lock()
            .push_back(MockSenderError::SomeError);

        let peer_id = MockPeerId::random();

        let message_1 = message(1);
        let message_2 = message(2);

        test_data
            .network
            .emit_event(MockEvent::StreamOpened(peer_id, Protocol::Validator));

        // We do this only to make sure that NotificationStreamOpened events are handled
        test_data.wait_for_events_handled().await;

        test_data
            .mock_io
            .legacy_messages_for_user
            .unbounded_send((
                message_1.clone(),
                DataCommand::SendTo(peer_id, Protocol::Validator),
            ))
            .unwrap();

        test_data
            .mock_io
            .legacy_messages_for_user
            .unbounded_send((
                message_2.clone(),
                DataCommand::SendTo(peer_id, Protocol::Validator),
            ))
            .unwrap();

        let expected = (message_2.encode(), peer_id, Protocol::Validator);

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
    async fn test_validator_data_command_send_to_error_many_peers() {
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

        let peer_ids: Vec<_> = (0..4).map(|_| MockPeerId::random()).collect();
        let message = message(1);

        peer_ids.iter().for_each(|peer_id| {
            test_data
                .network
                .emit_event(MockEvent::StreamOpened(*peer_id, Protocol::Validator));
        });

        // We do this only to make sure that NotificationStreamOpened events are handled
        test_data.wait_for_events_handled().await;

        peer_ids.iter().for_each(|peer_id| {
            test_data
                .mock_io
                .legacy_messages_for_user
                .unbounded_send((
                    message.clone(),
                    DataCommand::SendTo(*peer_id, Protocol::Validator),
                ))
                .unwrap();
        });

        let broadcasted_messages = HashSet::<_>::from_iter(
            test_data
                .network
                .send_message
                .1
                .lock()
                .await
                .by_ref()
                .take(all_authorities_n - closed_authorities_n)
                .collect::<Vec<_>>()
                .await
                .into_iter(),
        );

        let expected_messages = HashSet::from_iter(
            peer_ids
                .into_iter()
                .skip(closed_authorities_n)
                .map(|peer_id| (message.encode(), peer_id, Protocol::Validator)),
        );

        assert_eq!(broadcasted_messages, expected_messages);

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_generic_data_command_send_to() {
        let mut test_data = TestData::prepare().await;

        let peer_id = MockPeerId::random();

        let message = message(1);

        test_data
            .network
            .emit_event(MockEvent::StreamOpened(peer_id, Protocol::Generic));

        // We do this only to make sure that NotificationStreamOpened events are handled
        test_data.wait_for_events_handled().await;

        test_data
            .mock_io
            .legacy_messages_for_user
            .unbounded_send((
                message.clone(),
                DataCommand::SendTo(peer_id, Protocol::Generic),
            ))
            .unwrap();

        let expected = (message.encode(), peer_id, Protocol::Generic);

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
    async fn test_generic_create_sender_error_one_peer() {
        let mut test_data = TestData::prepare().await;

        test_data
            .network
            .create_sender_errors
            .lock()
            .push_back(MockSenderError::SomeError);

        let peer_id = MockPeerId::random();

        let message_1 = message(1);
        let message_2 = message(2);

        test_data
            .network
            .emit_event(MockEvent::StreamOpened(peer_id, Protocol::Generic));

        // We do this only to make sure that NotificationStreamOpened events are handled
        test_data.wait_for_events_handled().await;

        test_data
            .mock_io
            .legacy_messages_for_user
            .unbounded_send((
                message_1.clone(),
                DataCommand::SendTo(peer_id, Protocol::Generic),
            ))
            .unwrap();

        test_data
            .mock_io
            .legacy_messages_for_user
            .unbounded_send((
                message_2.clone(),
                DataCommand::SendTo(peer_id, Protocol::Generic),
            ))
            .unwrap();

        let expected = (message_2.encode(), peer_id, Protocol::Generic);

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
    async fn test_generic_create_sender_error_many_peers() {
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

        let peer_ids: Vec<_> = (0..4).map(|_| MockPeerId::random()).collect();
        let message = message(1);

        peer_ids.iter().for_each(|peer_id| {
            test_data
                .network
                .emit_event(MockEvent::StreamOpened(*peer_id, Protocol::Generic));
        });

        // We do this only to make sure that NotificationStreamOpened events are handled
        test_data.wait_for_events_handled().await;

        peer_ids.iter().for_each(|peer_id| {
            test_data
                .mock_io
                .legacy_messages_for_user
                .unbounded_send((
                    message.clone(),
                    DataCommand::SendTo(*peer_id, Protocol::Generic),
                ))
                .unwrap();
        });

        let broadcasted_messages = HashSet::<_>::from_iter(
            test_data
                .network
                .send_message
                .1
                .lock()
                .await
                .by_ref()
                .take(all_authorities_n - closed_authorities_n)
                .collect::<Vec<_>>()
                .await
                .into_iter(),
        );

        let expected_messages = HashSet::from_iter(
            peer_ids
                .into_iter()
                .skip(closed_authorities_n)
                .map(|peer_id| (message.encode(), peer_id, Protocol::Generic)),
        );

        assert_eq!(broadcasted_messages, expected_messages);

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_generic_data_command_send_to_error_one_peer() {
        let mut test_data = TestData::prepare().await;

        test_data
            .network
            .send_errors
            .lock()
            .push_back(MockSenderError::SomeError);

        let peer_id = MockPeerId::random();

        let message_1 = message(1);
        let message_2 = message(2);

        test_data
            .network
            .emit_event(MockEvent::StreamOpened(peer_id, Protocol::Generic));

        // We do this only to make sure that NotificationStreamOpened events are handled
        test_data.wait_for_events_handled().await;

        test_data
            .mock_io
            .legacy_messages_for_user
            .unbounded_send((
                message_1.clone(),
                DataCommand::SendTo(peer_id, Protocol::Generic),
            ))
            .unwrap();

        test_data
            .mock_io
            .legacy_messages_for_user
            .unbounded_send((
                message_2.clone(),
                DataCommand::SendTo(peer_id, Protocol::Generic),
            ))
            .unwrap();

        let expected = (message_2.encode(), peer_id, Protocol::Generic);

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
    async fn test_generic_data_command_send_to_error_many_peers() {
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

        let peer_ids: Vec<_> = (0..4).map(|_| MockPeerId::random()).collect();
        let message = message(1);

        peer_ids.iter().for_each(|peer_id| {
            test_data
                .network
                .emit_event(MockEvent::StreamOpened(*peer_id, Protocol::Generic));
        });

        // We do this only to make sure that NotificationStreamOpened events are handled
        test_data.wait_for_events_handled().await;

        peer_ids.iter().for_each(|peer_id| {
            test_data
                .mock_io
                .legacy_messages_for_user
                .unbounded_send((
                    message.clone(),
                    DataCommand::SendTo(*peer_id, Protocol::Generic),
                ))
                .unwrap();
        });

        let broadcasted_messages = HashSet::<_>::from_iter(
            test_data
                .network
                .send_message
                .1
                .lock()
                .await
                .by_ref()
                .take(all_authorities_n - closed_authorities_n)
                .collect::<Vec<_>>()
                .await
                .into_iter(),
        );

        let expected_messages = HashSet::from_iter(
            peer_ids
                .into_iter()
                .skip(closed_authorities_n)
                .map(|peer_id| (message.encode(), peer_id, Protocol::Generic)),
        );

        assert_eq!(broadcasted_messages, expected_messages);

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_notification_received() {
        let mut test_data = TestData::prepare().await;

        let message = message(1);

        test_data.network.emit_event(MockEvent::Messages(vec![(
            Protocol::Validator,
            NetworkData::encode(&message).into(),
        )]));

        assert_eq!(
            test_data
                .mock_io
                .legacy_messages_from_user
                .next()
                .await
                .expect("Should receive message"),
            message
        );

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_command_add_reserved() {
        let mut test_data = TestData::prepare().await;

        let (addresses, _) = MockNetworkIdentity::new().identity();

        test_data
            .mock_io
            .legacy_commands_for_manager
            .unbounded_send(ConnectionCommand::AddReserved(
                addresses.clone().into_iter().collect(),
            ))
            .unwrap();

        let expected = (addresses.into_iter().collect(), Protocol::Validator);

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
    async fn test_command_remove_reserved() {
        let mut test_data = TestData::prepare().await;

        let peer_id = MockPeerId::random();

        test_data
            .mock_io
            .legacy_commands_for_manager
            .unbounded_send(ConnectionCommand::DelReserved(
                iter::once(peer_id).collect(),
            ))
            .unwrap();

        let expected = (iter::once(peer_id).collect(), Protocol::Validator);

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
}
