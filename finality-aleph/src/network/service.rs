use std::{
    collections::{HashMap, HashSet},
    future::Future,
    iter,
};

use futures::{channel::mpsc, StreamExt};
use log::{debug, error, info, trace, warn};
use sc_service::SpawnTaskHandle;
use sc_utils::mpsc::{tracing_unbounded, TracingUnboundedReceiver, TracingUnboundedSender};
use tokio::time;

use crate::{
    network::{
        ConnectionCommand, Data, DataCommand, Event, EventStream, Multiaddress, Network,
        NetworkSender, Protocol,
    },
    STATUS_REPORT_INTERVAL,
};

/// A service managing all the direct interaction with the underlying network implementation. It
/// handles:
/// 1. Incoming network events
///   1. Messages are forwarded to the user.
///   2. Various forms of (dis)connecting, keeping track of all currently connected nodes.
/// 2. Commands from the network manager, modifying the reserved peer set.
/// 3. Outgoing messages, sending them out, using 1.2. to broadcast.
pub struct Service<N: Network, D: Data> {
    network: N,
    messages_from_user: mpsc::UnboundedReceiver<(D, DataCommand<N::PeerId>)>,
    messages_for_user: mpsc::UnboundedSender<D>,
    commands_from_manager: mpsc::UnboundedReceiver<ConnectionCommand<N::Multiaddress>>,
    generic_connected_peers: HashSet<N::PeerId>,
    validator_connected_peers: HashSet<N::PeerId>,
    generic_peer_senders: HashMap<N::PeerId, TracingUnboundedSender<D>>,
    validator_peer_senders: HashMap<N::PeerId, TracingUnboundedSender<D>>,
    spawn_handle: SpawnTaskHandle,
}

/// Input/output channels for the network service.
pub struct IO<D: Data, M: Multiaddress> {
    messages_from_user: mpsc::UnboundedReceiver<(D, DataCommand<M::PeerId>)>,
    messages_for_user: mpsc::UnboundedSender<D>,
    commands_from_manager: mpsc::UnboundedReceiver<ConnectionCommand<M>>,
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

impl<N: Network, D: Data> Service<N, D> {
    pub fn new(
        network: N,
        spawn_handle: SpawnTaskHandle,
        io: IO<D, N::Multiaddress>,
    ) -> Service<N, D> {
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
            generic_connected_peers: HashSet::new(),
            validator_connected_peers: HashSet::new(),
            generic_peer_senders: HashMap::new(),
            validator_peer_senders: HashMap::new(),
        }
    }

    fn get_sender(
        &mut self,
        peer: &N::PeerId,
        protocol: Protocol,
    ) -> Option<&mut TracingUnboundedSender<D>> {
        match protocol {
            Protocol::Generic => self.generic_peer_senders.get_mut(peer),
            Protocol::Validator => self.validator_peer_senders.get_mut(peer),
        }
    }

    fn peer_sender(
        &self,
        peer_id: N::PeerId,
        mut receiver: TracingUnboundedReceiver<D>,
        protocol: Protocol,
    ) -> impl Future<Output = ()> + Send + 'static {
        let network = self.network.clone();
        async move {
            let mut senders: HashMap<Protocol, N::NetworkSender> = HashMap::new();
            loop {
                if let Some(data) = receiver.next().await {
                    let sender = if let Some(sender) = senders.get(&protocol) {
                        sender
                    } else {
                        match network.sender(peer_id, protocol) {
                            Ok(sender) => senders.entry(protocol).or_insert(sender),
                            Err(e) => {
                                debug!(target: "aleph-network", "Failed creating sender. Dropping message: {}", e);
                                continue;
                            }
                        }
                    };
                    if let Err(e) = sender.send(data.encode()).await {
                        debug!(target: "aleph-network", "Failed sending data to peer. Dropping sender and message: {}", e);
                        senders.remove(&protocol);
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
        data: D,
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

    fn broadcast(&mut self, data: D) {
        for peer in self.generic_connected_peers.clone() {
            // We only broadcast authentication information in this sense, so we use the generic
            // Protocol.
            if let Err(e) = self.send_to_peer(data.clone(), peer, Protocol::Generic) {
                trace!(target: "aleph-network", "Failed to send broadcast to peer{:?}, {:?}", peer, e);
            }
        }
    }

    fn handle_network_event(
        &mut self,
        event: Event<N::Multiaddress>,
    ) -> Result<(), mpsc::TrySendError<D>> {
        use Event::*;
        match event {
            Connected(multiaddress) => {
                trace!(target: "aleph-network", "Connected event from address {:?}", multiaddress);
                self.network
                    .add_reserved(iter::once(multiaddress).collect(), Protocol::Generic);
            }
            Disconnected(peer) => {
                trace!(target: "aleph-network", "Disconnected event for peer {:?}", peer);
                self.network
                    .remove_reserved(iter::once(peer).collect(), Protocol::Generic);
            }
            StreamOpened(peer, protocol) => {
                trace!(target: "aleph-network", "StreamOpened event for peer {:?} and the protocol {:?}.", peer, protocol);
                let rx = match &protocol {
                    Protocol::Generic => {
                        let (tx, rx) = tracing_unbounded("mpsc_notification_stream_generic");
                        self.generic_connected_peers.insert(peer);
                        self.generic_peer_senders.insert(peer, tx);
                        rx
                    }
                    Protocol::Validator => {
                        let (tx, rx) = tracing_unbounded("mpsc_notification_stream_validator");
                        self.validator_connected_peers.insert(peer);
                        self.validator_peer_senders.insert(peer, tx);
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
                        self.generic_connected_peers.remove(&peer);
                        self.generic_peer_senders.remove(&peer);
                    }
                    Protocol::Validator => {
                        self.validator_connected_peers.remove(&peer);
                        self.validator_peer_senders.remove(&peer);
                    }
                }
            }
            Messages(messages) => {
                for data in messages.into_iter() {
                    match D::decode(&mut &data[..]) {
                        Ok(message) => self.messages_for_user.unbounded_send(message)?,
                        Err(e) => warn!(target: "aleph-network", "Error decoding message: {}", e),
                    }
                }
            }
        }
        Ok(())
    }

    fn on_manager_command(&self, command: ConnectionCommand<N::Multiaddress>) {
        use ConnectionCommand::*;
        match command {
            AddReserved(addresses) => {
                self.network.add_reserved(addresses, Protocol::Validator);
            }
            DelReserved(peers) => self.network.remove_reserved(peers, Protocol::Validator),
        }
    }

    fn on_user_command(&mut self, data: D, command: DataCommand<N::PeerId>) {
        use DataCommand::*;
        match command {
            Broadcast => self.broadcast(data),
            SendTo(peer, protocol) => {
                if let Err(e) = self.send_to_peer(data, peer, protocol) {
                    trace!(target: "aleph-network", "Failed to send data to peer{:?} via protocol {:?}, {:?}", peer, protocol, e);
                }
            }
        }
    }

    fn status_report(&self) {
        let mut status = String::from("Network status report: ");

        let peer_ids = self
            .validator_connected_peers
            .iter()
            .map(|peer_id| format!("{}", peer_id))
            .collect::<Vec<_>>()
            .join(", ");
        status.push_str(&format!(
            "validator connected peers - {:?} [{}]; ",
            self.validator_connected_peers.len(),
            peer_ids,
        ));

        status.push_str(&format!(
            "generic connected peers - {:?}; ",
            self.generic_connected_peers.len()
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
    use crate::network::{
        mock::{
            MockData, MockEvent, MockIO, MockMultiaddress, MockNetwork, MockNetworkIdentity,
            MockPeerId, MockSenderError,
        },
        NetworkIdentity, Protocol,
    };

    pub struct TestData {
        pub service_handle: JoinHandle<()>,
        pub exit_tx: oneshot::Sender<()>,
        pub network: MockNetwork<MockData>,
        pub mock_io: MockIO,
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
            self.network.close_channels().await;
        }

        // We do this only to make sure that NotificationStreamOpened/NotificationStreamClosed events are handled
        async fn wait_for_events_handled(&mut self) {
            let address = MockMultiaddress::random_with_id(MockPeerId::random());
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

    #[tokio::test]
    async fn test_sync_connected() {
        let mut test_data = TestData::prepare().await;

        let address = MockMultiaddress::random_with_id(MockPeerId::random());
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
                .map(|peer_id| (message.clone(), peer_id, Protocol::Generic)),
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
                        .map(move |m| (m.clone(), peer_id, Protocol::Generic))
                },
            ));

        assert_eq!(broadcasted_messages, expected_messages);

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_validator_data_command_send_to() {
        let mut test_data = TestData::prepare().await;

        let peer_id = MockPeerId::random();

        let message: Vec<u8> = vec![1, 2, 3];

        test_data
            .network
            .emit_event(MockEvent::StreamOpened(peer_id, Protocol::Validator));

        // We do this only to make sure that NotificationStreamOpened events are handled
        test_data.wait_for_events_handled().await;

        test_data
            .mock_io
            .messages_for_user
            .unbounded_send((
                message.clone(),
                DataCommand::SendTo(peer_id, Protocol::Validator),
            ))
            .unwrap();

        let expected = (message, peer_id, Protocol::Validator);

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

        let message_1: Vec<u8> = vec![1, 2, 3];
        let message_2: Vec<u8> = vec![4, 5, 6];

        test_data
            .network
            .emit_event(MockEvent::StreamOpened(peer_id, Protocol::Validator));

        // We do this only to make sure that NotificationStreamOpened events are handled
        test_data.wait_for_events_handled().await;

        test_data
            .mock_io
            .messages_for_user
            .unbounded_send((
                message_1.clone(),
                DataCommand::SendTo(peer_id, Protocol::Validator),
            ))
            .unwrap();

        test_data
            .mock_io
            .messages_for_user
            .unbounded_send((
                message_2.clone(),
                DataCommand::SendTo(peer_id, Protocol::Validator),
            ))
            .unwrap();

        let expected = (message_2, peer_id, Protocol::Validator);

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
        let message: Vec<u8> = vec![1, 2, 3];

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
                .messages_for_user
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
                .map(|peer_id| (message.clone(), peer_id, Protocol::Validator)),
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

        let message_1: Vec<u8> = vec![1, 2, 3];
        let message_2: Vec<u8> = vec![4, 5, 6];

        test_data
            .network
            .emit_event(MockEvent::StreamOpened(peer_id, Protocol::Validator));

        // We do this only to make sure that NotificationStreamOpened events are handled
        test_data.wait_for_events_handled().await;

        test_data
            .mock_io
            .messages_for_user
            .unbounded_send((
                message_1.clone(),
                DataCommand::SendTo(peer_id, Protocol::Validator),
            ))
            .unwrap();

        test_data
            .mock_io
            .messages_for_user
            .unbounded_send((
                message_2.clone(),
                DataCommand::SendTo(peer_id, Protocol::Validator),
            ))
            .unwrap();

        let expected = (message_2, peer_id, Protocol::Validator);

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
        let message: Vec<u8> = vec![1, 2, 3];

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
                .messages_for_user
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
                .map(|peer_id| (message.clone(), peer_id, Protocol::Validator)),
        );

        assert_eq!(broadcasted_messages, expected_messages);

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_generic_data_command_send_to() {
        let mut test_data = TestData::prepare().await;

        let peer_id = MockPeerId::random();

        let message: Vec<u8> = vec![1, 2, 3];

        test_data
            .network
            .emit_event(MockEvent::StreamOpened(peer_id, Protocol::Generic));

        // We do this only to make sure that NotificationStreamOpened events are handled
        test_data.wait_for_events_handled().await;

        test_data
            .mock_io
            .messages_for_user
            .unbounded_send((
                message.clone(),
                DataCommand::SendTo(peer_id, Protocol::Generic),
            ))
            .unwrap();

        let expected = (message, peer_id, Protocol::Generic);

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

        let message_1: Vec<u8> = vec![1, 2, 3];
        let message_2: Vec<u8> = vec![4, 5, 6];

        test_data
            .network
            .emit_event(MockEvent::StreamOpened(peer_id, Protocol::Generic));

        // We do this only to make sure that NotificationStreamOpened events are handled
        test_data.wait_for_events_handled().await;

        test_data
            .mock_io
            .messages_for_user
            .unbounded_send((
                message_1.clone(),
                DataCommand::SendTo(peer_id, Protocol::Generic),
            ))
            .unwrap();

        test_data
            .mock_io
            .messages_for_user
            .unbounded_send((
                message_2.clone(),
                DataCommand::SendTo(peer_id, Protocol::Generic),
            ))
            .unwrap();

        let expected = (message_2, peer_id, Protocol::Generic);

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
        let message: Vec<u8> = vec![1, 2, 3];

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
                .messages_for_user
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
                .map(|peer_id| (message.clone(), peer_id, Protocol::Generic)),
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

        let message_1: Vec<u8> = vec![1, 2, 3];
        let message_2: Vec<u8> = vec![4, 5, 6];

        test_data
            .network
            .emit_event(MockEvent::StreamOpened(peer_id, Protocol::Generic));

        // We do this only to make sure that NotificationStreamOpened events are handled
        test_data.wait_for_events_handled().await;

        test_data
            .mock_io
            .messages_for_user
            .unbounded_send((
                message_1.clone(),
                DataCommand::SendTo(peer_id, Protocol::Generic),
            ))
            .unwrap();

        test_data
            .mock_io
            .messages_for_user
            .unbounded_send((
                message_2.clone(),
                DataCommand::SendTo(peer_id, Protocol::Generic),
            ))
            .unwrap();

        let expected = (message_2, peer_id, Protocol::Generic);

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
        let message: Vec<u8> = vec![1, 2, 3];

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
                .messages_for_user
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
                .map(|peer_id| (message.clone(), peer_id, Protocol::Generic)),
        );

        assert_eq!(broadcasted_messages, expected_messages);

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_notification_received() {
        let mut test_data = TestData::prepare().await;

        let message: Vec<u8> = vec![1, 2, 3];

        test_data
            .network
            .emit_event(MockEvent::Messages(vec![Vec::encode(&message).into()]));

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
        let mut test_data = TestData::prepare().await;

        let (addresses, _) = MockNetworkIdentity::new().identity();

        test_data
            .mock_io
            .commands_for_manager
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
            .commands_for_manager
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
