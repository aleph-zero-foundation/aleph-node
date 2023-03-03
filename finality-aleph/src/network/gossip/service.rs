use std::{
    collections::{HashMap, HashSet},
    fmt::{Debug, Display, Error as FmtError, Formatter},
    future::Future,
    hash::Hash,
};

use futures::{channel::mpsc, StreamExt};
use log::{debug, error, info, trace, warn};
use rand::{seq::IteratorRandom, thread_rng};
use sc_service::SpawnTaskHandle;
use sc_utils::mpsc::{tracing_unbounded, TracingUnboundedReceiver, TracingUnboundedSender};
use tokio::time;

const QUEUE_SIZE_WARNING: i64 = 1_000;

use crate::{
    network::{
        gossip::{Event, EventStream, Network, NetworkSender, Protocol, RawNetwork},
        Data,
    },
    STATUS_REPORT_INTERVAL,
};

enum Command<D: Data, P: Clone + Debug + Eq + Hash + Send + 'static> {
    Send(D, P, Protocol),
    SendToRandom(D, HashSet<P>, Protocol),
    Broadcast(D, Protocol),
}

/// A service managing all the direct interaction with the underlying network implementation. It
/// handles:
/// 1. Incoming network events
///   1. Messages are forwarded to the user.
///   2. Various forms of (dis)connecting, keeping track of all currently connected nodes.
/// 3. Outgoing messages, sending them out, using 1.2. to broadcast.
pub struct Service<N: RawNetwork, D: Data> {
    network: N,
    messages_from_user: mpsc::UnboundedReceiver<Command<D, N::PeerId>>,
    messages_for_authentication_user: mpsc::UnboundedSender<(D, N::PeerId)>,
    messages_for_block_sync_user: mpsc::UnboundedSender<(D, N::PeerId)>,
    authentication_connected_peers: HashSet<N::PeerId>,
    authentication_peer_senders: HashMap<N::PeerId, TracingUnboundedSender<D>>,
    block_sync_connected_peers: HashSet<N::PeerId>,
    block_sync_peer_senders: HashMap<N::PeerId, TracingUnboundedSender<D>>,
    spawn_handle: SpawnTaskHandle,
}

struct ServiceInterface<D: Data, P: Clone + Debug + Eq + Hash + Send + 'static> {
    protocol: Protocol,
    messages_from_service: mpsc::UnboundedReceiver<(D, P)>,
    messages_for_service: mpsc::UnboundedSender<Command<D, P>>,
}

/// What can go wrong when receiving or sending data.
#[derive(Debug)]
pub enum Error {
    ServiceStopped,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use Error::*;
        match self {
            ServiceStopped => {
                write!(f, "gossip network service stopped")
            }
        }
    }
}

#[async_trait::async_trait]
impl<D: Data, P: Clone + Debug + Eq + Hash + Send + 'static> Network<D> for ServiceInterface<D, P> {
    type Error = Error;
    type PeerId = P;

    fn send_to(&mut self, data: D, peer_id: Self::PeerId) -> Result<(), Self::Error> {
        self.messages_for_service
            .unbounded_send(Command::Send(data, peer_id, self.protocol))
            .map_err(|_| Error::ServiceStopped)
    }

    fn send_to_random(
        &mut self,
        data: D,
        peer_ids: HashSet<Self::PeerId>,
    ) -> Result<(), Self::Error> {
        self.messages_for_service
            .unbounded_send(Command::SendToRandom(data, peer_ids, self.protocol))
            .map_err(|_| Error::ServiceStopped)
    }

    fn broadcast(&mut self, data: D) -> Result<(), Self::Error> {
        self.messages_for_service
            .unbounded_send(Command::Broadcast(data, self.protocol))
            .map_err(|_| Error::ServiceStopped)
    }

    async fn next(&mut self) -> Result<(D, Self::PeerId), Self::Error> {
        self.messages_from_service
            .next()
            .await
            .ok_or(Error::ServiceStopped)
    }
}

#[derive(Debug)]
enum SendError {
    MissingSender,
    SendingFailed,
}

impl<N: RawNetwork, D: Data> Service<N, D> {
    pub fn new(
        network: N,
        spawn_handle: SpawnTaskHandle,
    ) -> (
        Service<N, D>,
        impl Network<D, Error = Error, PeerId = N::PeerId>,
        impl Network<D, Error = Error, PeerId = N::PeerId>,
    ) {
        let (messages_for_authentication_user, messages_from_authentication_service) =
            mpsc::unbounded();
        let (messages_for_block_sync_user, messages_from_block_sync_service) = mpsc::unbounded();
        let (messages_for_service, messages_from_user) = mpsc::unbounded();
        (
            Service {
                network,
                messages_from_user,
                messages_for_authentication_user,
                messages_for_block_sync_user,
                spawn_handle,
                authentication_connected_peers: HashSet::new(),
                authentication_peer_senders: HashMap::new(),
                block_sync_connected_peers: HashSet::new(),
                block_sync_peer_senders: HashMap::new(),
            },
            ServiceInterface {
                protocol: Protocol::Authentication,
                messages_from_service: messages_from_authentication_service,
                messages_for_service: messages_for_service.clone(),
            },
            ServiceInterface {
                protocol: Protocol::BlockSync,
                messages_from_service: messages_from_block_sync_service,
                messages_for_service,
            },
        )
    }

    fn get_sender(
        &mut self,
        peer: &N::PeerId,
        protocol: Protocol,
    ) -> Option<&mut TracingUnboundedSender<D>> {
        match protocol {
            Protocol::Authentication => self.authentication_peer_senders.get_mut(peer),
            Protocol::BlockSync => self.block_sync_peer_senders.get_mut(peer),
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

    fn send(&mut self, data: D, peer_id: N::PeerId, protocol: Protocol) {
        if let Err(e) = self.send_to_peer(data, peer_id.clone(), protocol) {
            trace!(target: "aleph-network", "Failed to send to peer{:?}, {:?}", peer_id, e);
        }
    }

    fn protocol_peers(&self, protocol: Protocol) -> &HashSet<N::PeerId> {
        match protocol {
            Protocol::Authentication => &self.authentication_connected_peers,
            Protocol::BlockSync => &self.block_sync_connected_peers,
        }
    }

    fn random_peer<'a>(
        &'a self,
        peer_ids: &'a HashSet<N::PeerId>,
        protocol: Protocol,
    ) -> Option<&'a N::PeerId> {
        peer_ids
            .intersection(self.protocol_peers(protocol))
            .into_iter()
            .choose(&mut thread_rng())
            .or_else(|| {
                self.protocol_peers(protocol)
                    .iter()
                    .choose(&mut thread_rng())
            })
    }

    fn send_to_random(&mut self, data: D, peer_ids: HashSet<N::PeerId>, protocol: Protocol) {
        let peer_id = match self.random_peer(&peer_ids, protocol) {
            Some(peer_id) => peer_id.clone(),
            None => {
                trace!(target: "aleph-network", "Failed to send to random peer, no peers are available.");
                return;
            }
        };
        self.send(data, peer_id, protocol);
    }

    fn broadcast(&mut self, data: D, protocol: Protocol) {
        let peers = self.protocol_peers(protocol).clone();
        for peer in peers {
            self.send(data.clone(), peer, protocol);
        }
    }

    fn handle_network_event(
        &mut self,
        event: Event<N::PeerId>,
    ) -> Result<(), mpsc::TrySendError<(D, N::PeerId)>> {
        use Event::*;
        match event {
            StreamOpened(peer, protocol) => {
                trace!(target: "aleph-network", "StreamOpened event for peer {:?} and the protocol {:?}.", peer, protocol);
                let rx = match &protocol {
                    Protocol::Authentication => {
                        let (tx, rx) = tracing_unbounded(
                            "mpsc_notification_stream_authentication",
                            QUEUE_SIZE_WARNING,
                        );
                        self.authentication_connected_peers.insert(peer.clone());
                        self.authentication_peer_senders.insert(peer.clone(), tx);
                        rx
                    }
                    Protocol::BlockSync => {
                        let (tx, rx) = tracing_unbounded(
                            "mpsc_notification_stream_block_sync",
                            QUEUE_SIZE_WARNING,
                        );
                        self.block_sync_connected_peers.insert(peer.clone());
                        self.block_sync_peer_senders.insert(peer.clone(), tx);
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
                    Protocol::BlockSync => {
                        self.block_sync_connected_peers.remove(&peer);
                        self.block_sync_peer_senders.remove(&peer);
                    }
                }
            }
            Messages(peer_id, messages) => {
                for (protocol, data) in messages.into_iter() {
                    match protocol {
                        Protocol::Authentication => match D::decode(&mut &data[..]) {
                            Ok(data) => self
                                .messages_for_authentication_user
                                .unbounded_send((data, peer_id.clone()))?,
                            Err(e) => {
                                warn!(target: "aleph-network", "Error decoding authentication protocol message: {}", e)
                            }
                        },
                        // This is a bit of a placeholder for now, as we are not yet using this
                        // protocol. In the future we will not be using the same D as above.
                        Protocol::BlockSync => match D::decode(&mut &data[..]) {
                            Ok(data) => self
                                .messages_for_block_sync_user
                                .unbounded_send((data, peer_id.clone()))?,
                            Err(e) => {
                                warn!(target: "aleph-network", "Error decoding block sync protocol message: {}", e)
                            }
                        },
                    };
                }
            }
        }
        Ok(())
    }

    fn status_report(&self) {
        let mut status = String::from("Network status report: ");

        status.push_str(&format!(
            "authentication connected peers - {:?}; ",
            self.authentication_connected_peers.len()
        ));
        status.push_str(&format!(
            "block sync connected peers - {:?}; ",
            self.block_sync_connected_peers.len()
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
                maybe_message = self.messages_from_user.next() => match maybe_message {
                    Some(Command::Broadcast(message, protocol)) => self.broadcast(message, protocol),
                    Some(Command::SendToRandom(message, peer_ids, protocol)) => self.send_to_random(message, peer_ids, protocol),
                    Some(Command::Send(message, peer_id, protocol)) => self.send(message, peer_id, protocol),
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
    use std::{collections::HashSet, iter};

    use codec::Encode;
    use futures::channel::oneshot;
    use sc_service::TaskManager;
    use tokio::runtime::Handle;

    use super::{Error, Service};
    use crate::network::{
        clique::mock::{random_peer_id, MockPublicKey},
        gossip::{
            mock::{MockEvent, MockRawNetwork, MockSenderError},
            Network,
        },
        mock::MockData,
        Protocol,
    };

    const PROTOCOL: Protocol = Protocol::Authentication;

    pub struct TestData {
        pub network: MockRawNetwork,
        gossip_network: Box<dyn Network<MockData, Error = Error, PeerId = MockPublicKey>>,
        pub service: Service<MockRawNetwork, MockData>,
        // `TaskManager` can't be dropped for `SpawnTaskHandle` to work
        _task_manager: TaskManager,
    }

    impl TestData {
        async fn prepare() -> Self {
            let task_manager = TaskManager::new(Handle::current(), None).unwrap();

            // Event stream will never be taken, so we can drop the receiver
            let (event_stream_oneshot_tx, _) = oneshot::channel();

            // Prepare service
            let network = MockRawNetwork::new(event_stream_oneshot_tx);
            let (service, gossip_network, _) =
                Service::new(network.clone(), task_manager.spawn_handle());
            let gossip_network = Box::new(gossip_network);

            // `TaskManager` needs to be passed, so sender threads are running in background.
            Self {
                network,
                service,
                gossip_network,
                _task_manager: task_manager,
            }
        }

        async fn cleanup(self) {
            self.network.close_channels().await;
        }
    }

    #[async_trait::async_trait]
    impl Network<MockData> for TestData {
        type Error = Error;
        type PeerId = MockPublicKey;

        fn send_to(&mut self, data: MockData, peer_id: Self::PeerId) -> Result<(), Self::Error> {
            self.gossip_network.send_to(data, peer_id)
        }

        fn send_to_random(
            &mut self,
            data: MockData,
            peer_ids: HashSet<Self::PeerId>,
        ) -> Result<(), Self::Error> {
            self.gossip_network.send_to_random(data, peer_ids)
        }

        fn broadcast(&mut self, data: MockData) -> Result<(), Self::Error> {
            self.gossip_network.broadcast(data)
        }

        async fn next(&mut self) -> Result<(MockData, Self::PeerId), Self::Error> {
            self.gossip_network.next().await
        }
    }

    fn message(i: u8) -> MockData {
        MockData::new(i.into(), 3)
    }

    #[tokio::test]
    async fn test_notification_stream_opened() {
        let mut test_data = TestData::prepare().await;

        let peer_ids: Vec<_> = (0..3).map(|_| random_peer_id()).collect();

        peer_ids.iter().for_each(|peer_id| {
            test_data
                .service
                .handle_network_event(MockEvent::StreamOpened(peer_id.clone(), PROTOCOL))
                .expect("Should handle");
        });

        let message = message(1);
        test_data.service.broadcast(message.clone(), PROTOCOL);

        let broadcasted_messages = HashSet::<_>::from_iter(
            test_data
                .network
                .send_message
                .take(peer_ids.len())
                .await
                .into_iter(),
        );

        let expected_messages = HashSet::from_iter(
            peer_ids
                .into_iter()
                .map(|peer_id| (message.clone().encode(), peer_id, PROTOCOL)),
        );

        assert_eq!(broadcasted_messages, expected_messages);

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_notification_stream_closed() {
        let mut test_data = TestData::prepare().await;

        let peer_ids: Vec<_> = (0..3).map(|_| random_peer_id()).collect();
        let opened_authorities_n = 2;

        peer_ids.iter().for_each(|peer_id| {
            test_data
                .service
                .handle_network_event(MockEvent::StreamOpened(peer_id.clone(), PROTOCOL))
                .expect("Should handle");
        });

        peer_ids
            .iter()
            .skip(opened_authorities_n)
            .for_each(|peer_id| {
                test_data
                    .service
                    .handle_network_event(MockEvent::StreamClosed(peer_id.clone(), PROTOCOL))
                    .expect("Should handle");
            });

        let message = message(1);
        test_data.service.broadcast(message.clone(), PROTOCOL);

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
                .map(|peer_id| (message.clone().encode(), peer_id, PROTOCOL)),
        );

        assert_eq!(broadcasted_messages, expected_messages);

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_create_sender_error() {
        let mut test_data = TestData::prepare().await;

        test_data
            .network
            .create_sender_errors
            .lock()
            .push_back(MockSenderError);

        let peer_id = random_peer_id();

        let message_1 = message(1);
        let message_2 = message(4);

        test_data
            .service
            .handle_network_event(MockEvent::StreamOpened(peer_id.clone(), PROTOCOL))
            .expect("Should handle");

        test_data.service.broadcast(message_1, PROTOCOL);

        test_data.service.broadcast(message_2.clone(), PROTOCOL);

        let expected = (message_2.encode(), peer_id, PROTOCOL);

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
            .push_back(MockSenderError);

        let peer_id = random_peer_id();

        let message_1 = message(1);
        let message_2 = message(4);

        test_data
            .service
            .handle_network_event(MockEvent::StreamOpened(peer_id.clone(), PROTOCOL))
            .expect("Should handle");

        test_data.service.broadcast(message_1, PROTOCOL);

        test_data.service.broadcast(message_2.clone(), PROTOCOL);

        let expected = (message_2.encode(), peer_id, PROTOCOL);

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

        let message = message(1);

        let peer_id = random_peer_id();
        test_data
            .service
            .handle_network_event(MockEvent::Messages(
                peer_id.clone(),
                vec![(PROTOCOL, message.clone().encode().into())],
            ))
            .expect("Should handle");

        let (received_message, received_peer_id) =
            test_data.next().await.expect("Should receive message");
        assert_eq!(received_message, message);
        assert_eq!(received_peer_id, peer_id);

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_send_to_connected() {
        let mut test_data = TestData::prepare().await;

        let peer_id = random_peer_id();

        let message = message(1);

        test_data
            .service
            .handle_network_event(MockEvent::StreamOpened(peer_id.clone(), PROTOCOL))
            .expect("Should handle");

        test_data
            .service
            .send(message.clone(), peer_id.clone(), PROTOCOL);

        let expected = (message.encode(), peer_id, PROTOCOL);

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
    async fn test_no_send_to_disconnected() {
        let mut test_data = TestData::prepare().await;

        let peer_id = random_peer_id();

        let message = message(1);

        test_data.service.send(message, peer_id, PROTOCOL);

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_send_to_random_connected() {
        let mut test_data = TestData::prepare().await;

        let peer_id = random_peer_id();

        let message = message(1);

        test_data
            .service
            .handle_network_event(MockEvent::StreamOpened(peer_id.clone(), PROTOCOL))
            .expect("Should handle");

        test_data.service.send_to_random(
            message.clone(),
            iter::once(peer_id.clone()).collect(),
            PROTOCOL,
        );

        let expected = (message.encode(), peer_id, PROTOCOL);

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
    async fn test_send_to_random_disconnected() {
        let mut test_data = TestData::prepare().await;

        let peer_id = random_peer_id();
        let other_peer_id = random_peer_id();

        let message = message(1);

        test_data
            .service
            .handle_network_event(MockEvent::StreamOpened(other_peer_id.clone(), PROTOCOL))
            .expect("Should handle");

        test_data.service.send_to_random(
            message.clone(),
            iter::once(peer_id.clone()).collect(),
            PROTOCOL,
        );

        let expected = (message.encode(), other_peer_id, PROTOCOL);

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
}
