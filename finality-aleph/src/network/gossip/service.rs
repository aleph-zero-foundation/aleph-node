use std::{
    collections::{HashMap, HashSet},
    fmt::{Debug, Display, Error as FmtError, Formatter},
    future::Future,
    hash::Hash,
};

use futures::{channel::mpsc, StreamExt};
use log::{debug, error, info, trace, warn};
use network_clique::SpawnHandleT;
use rand::{seq::IteratorRandom, thread_rng};
use sc_utils::mpsc::{tracing_unbounded, TracingUnboundedReceiver, TracingUnboundedSender};
use tokio::time;

const QUEUE_SIZE_WARNING: i64 = 1_000;

use crate::{
    metrics::Key,
    network::{
        gossip::{Event, EventStream, Network, NetworkSender, Protocol, RawNetwork},
        Data,
    },
    Metrics, SpawnHandle, STATUS_REPORT_INTERVAL,
};

enum Command<D: Data, P: Clone + Debug + Eq + Hash + Send + 'static> {
    Send(D, P),
    SendToRandom(D, HashSet<P>),
    Broadcast(D),
}

/// A service managing all the direct interaction with the underlying network implementation. It
/// handles:
/// 1. Incoming network events
///   1. Messages are forwarded to the user.
///   2. Various forms of (dis)connecting, keeping track of all currently connected nodes.
/// 3. Outgoing messages, sending them out, using 1.2. to broadcast.
pub struct Service<N: RawNetwork, AD: Data, BSD: Data, H: Key> {
    network: N,
    messages_from_authentication_user: mpsc::UnboundedReceiver<Command<AD, N::PeerId>>,
    messages_from_block_sync_user: mpsc::UnboundedReceiver<Command<BSD, N::PeerId>>,
    messages_for_authentication_user: mpsc::UnboundedSender<(AD, N::PeerId)>,
    messages_for_block_sync_user: mpsc::UnboundedSender<(BSD, N::PeerId)>,
    authentication_connected_peers: HashSet<N::PeerId>,
    authentication_peer_senders: HashMap<N::PeerId, TracingUnboundedSender<AD>>,
    block_sync_connected_peers: HashSet<N::PeerId>,
    block_sync_peer_senders: HashMap<N::PeerId, TracingUnboundedSender<BSD>>,
    spawn_handle: SpawnHandle,
    metrics: Metrics<H>,
}

struct ServiceInterface<D: Data, P: Clone + Debug + Eq + Hash + Send + 'static> {
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
            .unbounded_send(Command::Send(data, peer_id))
            .map_err(|_| Error::ServiceStopped)
    }

    fn send_to_random(
        &mut self,
        data: D,
        peer_ids: HashSet<Self::PeerId>,
    ) -> Result<(), Self::Error> {
        self.messages_for_service
            .unbounded_send(Command::SendToRandom(data, peer_ids))
            .map_err(|_| Error::ServiceStopped)
    }

    fn broadcast(&mut self, data: D) -> Result<(), Self::Error> {
        self.messages_for_service
            .unbounded_send(Command::Broadcast(data))
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

impl<N: RawNetwork, AD: Data, BSD: Data, H: Key> Service<N, AD, BSD, H> {
    pub fn new(
        network: N,
        spawn_handle: SpawnHandle,
        metrics: Metrics<H>,
    ) -> (
        Self,
        impl Network<AD, Error = Error, PeerId = N::PeerId>,
        impl Network<BSD, Error = Error, PeerId = N::PeerId>,
    ) {
        let (messages_for_authentication_user, messages_from_authentication_service) =
            mpsc::unbounded();
        let (messages_for_block_sync_user, messages_from_block_sync_service) = mpsc::unbounded();
        let (messages_for_authentication_service, messages_from_authentication_user) =
            mpsc::unbounded();
        let (messages_for_block_sync_service, messages_from_block_sync_user) = mpsc::unbounded();
        (
            Service {
                network,
                messages_from_authentication_user,
                messages_from_block_sync_user,
                messages_for_authentication_user,
                messages_for_block_sync_user,
                spawn_handle,
                authentication_connected_peers: HashSet::new(),
                authentication_peer_senders: HashMap::new(),
                block_sync_connected_peers: HashSet::new(),
                block_sync_peer_senders: HashMap::new(),
                metrics,
            },
            ServiceInterface {
                messages_from_service: messages_from_authentication_service,
                messages_for_service: messages_for_authentication_service,
            },
            ServiceInterface {
                messages_from_service: messages_from_block_sync_service,
                messages_for_service: messages_for_block_sync_service,
            },
        )
    }

    fn get_authentication_sender(
        &mut self,
        peer: &N::PeerId,
    ) -> Option<&mut TracingUnboundedSender<AD>> {
        self.authentication_peer_senders.get_mut(peer)
    }

    fn get_block_sync_sender(
        &mut self,
        peer: &N::PeerId,
    ) -> Option<&mut TracingUnboundedSender<BSD>> {
        self.block_sync_peer_senders.get_mut(peer)
    }

    fn peer_sender<D: Data>(
        &self,
        peer_id: N::PeerId,
        mut receiver: TracingUnboundedReceiver<D>,
        protocol: Protocol,
    ) -> impl Future<Output = ()> + Send + 'static {
        let network = self.network.clone();
        let metrics = self.metrics.clone();
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
                    let maybe_timer = metrics.start_sending_in(protocol);
                    if let Err(e) = s.send(data.encode()).await {
                        debug!(target: "aleph-network", "Failed sending data to peer. Dropping sender and message: {}", e);
                        sender = None;
                    }
                    if let Some(timer) = maybe_timer {
                        timer.observe_duration();
                    }
                } else {
                    debug!(target: "aleph-network", "Sender was dropped for peer {:?}. Peer sender exiting.", peer_id);
                    return;
                }
            }
        }
    }

    fn send_to_authentication_peer(&mut self, data: AD, peer: N::PeerId) -> Result<(), SendError> {
        match self.get_authentication_sender(&peer) {
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

    fn send_to_block_sync_peer(&mut self, data: BSD, peer: N::PeerId) -> Result<(), SendError> {
        match self.get_block_sync_sender(&peer) {
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

    fn send_authentication_data(&mut self, data: AD, peer_id: N::PeerId) {
        if let Err(e) = self.send_to_authentication_peer(data, peer_id.clone()) {
            trace!(target: "aleph-network", "Failed to send to peer{:?}, {:?}", peer_id, e);
        }
    }

    fn send_block_sync_data(&mut self, data: BSD, peer_id: N::PeerId) {
        if let Err(e) = self.send_to_block_sync_peer(data, peer_id.clone()) {
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

    fn send_to_random_authentication(&mut self, data: AD, peer_ids: HashSet<N::PeerId>) {
        let peer_id = match self.random_peer(&peer_ids, Protocol::Authentication) {
            Some(peer_id) => peer_id.clone(),
            None => {
                trace!(target: "aleph-network", "Failed to send authentication message to random peer, no peers are available.");
                return;
            }
        };
        self.send_authentication_data(data, peer_id);
    }

    fn send_to_random_block_sync(&mut self, data: BSD, peer_ids: HashSet<N::PeerId>) {
        let peer_id = match self.random_peer(&peer_ids, Protocol::BlockSync) {
            Some(peer_id) => peer_id.clone(),
            None => {
                trace!(target: "aleph-network", "Failed to send block sync message to random peer, no peers are available.");
                return;
            }
        };
        self.send_block_sync_data(data, peer_id);
    }

    fn broadcast_authentication(&mut self, data: AD) {
        let peers = self.protocol_peers(Protocol::Authentication).clone();
        for peer in peers {
            self.send_authentication_data(data.clone(), peer);
        }
    }

    fn broadcast_block_sync(&mut self, data: BSD) {
        let peers = self.protocol_peers(Protocol::BlockSync).clone();
        for peer in peers {
            self.send_block_sync_data(data.clone(), peer);
        }
    }

    fn handle_network_event(&mut self, event: Event<N::PeerId>) -> Result<(), ()> {
        use Event::*;
        match event {
            StreamOpened(peer, protocol) => {
                trace!(target: "aleph-network", "StreamOpened event for peer {:?} and the protocol {:?}.", peer, protocol);
                match protocol {
                    Protocol::Authentication => {
                        let (tx, rx) = tracing_unbounded(
                            "mpsc_notification_stream_authentication",
                            QUEUE_SIZE_WARNING,
                        );
                        self.authentication_connected_peers.insert(peer.clone());
                        self.authentication_peer_senders.insert(peer.clone(), tx);
                        self.spawn_handle.spawn(
                            "aleph/network/authentication_peer_sender",
                            self.peer_sender(peer, rx, Protocol::Authentication),
                        );
                    }
                    Protocol::BlockSync => {
                        let (tx, rx) = tracing_unbounded(
                            "mpsc_notification_stream_block_sync",
                            QUEUE_SIZE_WARNING,
                        );
                        self.block_sync_connected_peers.insert(peer.clone());
                        self.block_sync_peer_senders.insert(peer.clone(), tx);
                        self.spawn_handle.spawn(
                            "aleph/network/sync_peer_sender",
                            self.peer_sender(peer, rx, Protocol::BlockSync),
                        );
                    }
                };
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
                        Protocol::Authentication => match AD::decode(&mut &data[..]) {
                            Ok(data) => self
                                .messages_for_authentication_user
                                .unbounded_send((data, peer_id.clone()))
                                .map_err(|_| ())?,
                            Err(e) => {
                                warn!(target: "aleph-network", "Error decoding authentication protocol message: {}", e)
                            }
                        },
                        Protocol::BlockSync => match BSD::decode(&mut &data[..]) {
                            Ok(data) => self
                                .messages_for_block_sync_user
                                .unbounded_send((data, peer_id.clone()))
                                .map_err(|_| ())?,
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
                    Some(event) => if self.handle_network_event(event).is_err() {
                        error!(target: "aleph-network", "Cannot forward messages to user.");
                        return;
                    },
                    None => {
                        error!(target: "aleph-network", "Network event stream ended.");
                        return;
                    }
                },
                maybe_message = self.messages_from_authentication_user.next() => match maybe_message {
                    Some(Command::Broadcast(message)) => self.broadcast_authentication(message),
                    Some(Command::SendToRandom(message, peer_ids)) => self.send_to_random_authentication(message, peer_ids),
                    Some(Command::Send(message, peer_id)) => self.send_authentication_data(message, peer_id),
                    None => {
                        error!(target: "aleph-network", "Authentication user message stream ended.");
                        return;
                    }
                },
                maybe_message = self.messages_from_block_sync_user.next() => match maybe_message {
                    Some(Command::Broadcast(message)) => self.broadcast_block_sync(message),
                    Some(Command::SendToRandom(message, peer_ids)) => self.send_to_random_block_sync(message, peer_ids),
                    Some(Command::Send(message, peer_id)) => self.send_block_sync_data(message, peer_id),
                    None => {
                        error!(target: "aleph-network", "Block sync user message stream ended.");
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
    use network_clique::mock::{random_peer_id, MockPublicKey};
    use sc_service::TaskManager;
    use tokio::runtime::Handle;

    use super::{Error, SendError, Service};
    use crate::{
        metrics::Metrics,
        network::{
            gossip::{
                mock::{MockEvent, MockRawNetwork, MockSenderError},
                Network,
            },
            mock::MockData,
            Protocol,
        },
        testing::mocks::THash,
    };

    const PROTOCOL: Protocol = Protocol::Authentication;

    pub struct TestData {
        pub network: MockRawNetwork,
        gossip_network: Box<dyn Network<MockData, Error = Error, PeerId = MockPublicKey>>,
        pub service: Service<MockRawNetwork, MockData, MockData, THash>,
        // `TaskManager` can't be dropped for `SpawnTaskHandle` to work
        _task_manager: TaskManager,
        // If we drop the sync network, the underlying network service dies, stopping the whole
        // network.
        _other_network: Box<dyn Network<MockData, Error = Error, PeerId = MockPublicKey>>,
    }

    impl TestData {
        fn prepare() -> Self {
            let task_manager = TaskManager::new(Handle::current(), None).unwrap();

            // Event stream will never be taken, so we can drop the receiver
            let (event_stream_oneshot_tx, _) = oneshot::channel();

            // Prepare service
            let network = MockRawNetwork::new(event_stream_oneshot_tx);
            let (service, gossip_network, other_network) = Service::new(
                network.clone(),
                task_manager.spawn_handle().into(),
                Metrics::<THash>::noop(),
            );
            let gossip_network = Box::new(gossip_network);
            let other_network = Box::new(other_network);

            // `TaskManager` needs to be passed, so sender threads are running in background.
            Self {
                network,
                service,
                gossip_network,
                _task_manager: task_manager,
                _other_network: other_network,
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
        let mut test_data = TestData::prepare();

        let peer_ids: Vec<_> = (0..3).map(|_| random_peer_id()).collect();

        peer_ids.iter().for_each(|peer_id| {
            test_data
                .service
                .handle_network_event(MockEvent::StreamOpened(peer_id.clone(), PROTOCOL))
                .expect("Should handle");
        });

        let message = message(1);
        test_data.service.broadcast_authentication(message.clone());

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
        let mut test_data = TestData::prepare();

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
        test_data.service.broadcast_authentication(message.clone());

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
        let mut test_data = TestData::prepare();

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

        test_data.service.broadcast_authentication(message_1);

        test_data
            .service
            .broadcast_authentication(message_2.clone());

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
        let mut test_data = TestData::prepare();

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

        test_data.service.broadcast_authentication(message_1);

        test_data
            .service
            .broadcast_authentication(message_2.clone());

        let expected = (message_2.encode(), peer_id, PROTOCOL);

        println!("just before");
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
        let mut test_data = TestData::prepare();

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
        let mut test_data = TestData::prepare();

        let peer_id = random_peer_id();

        let message = message(1);

        test_data
            .service
            .handle_network_event(MockEvent::StreamOpened(peer_id.clone(), PROTOCOL))
            .expect("Should handle");

        test_data
            .service
            .send_to_authentication_peer(message.clone(), peer_id.clone())
            .expect("interface works");

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
        let mut test_data = TestData::prepare();

        let peer_id = random_peer_id();

        let message = message(1);

        assert!(matches!(
            test_data
                .service
                .send_to_authentication_peer(message, peer_id),
            Err(SendError::MissingSender)
        ));

        test_data.cleanup().await
    }

    #[tokio::test]
    async fn test_send_to_random_connected() {
        let mut test_data = TestData::prepare();

        let peer_id = random_peer_id();

        let message = message(1);

        test_data
            .service
            .handle_network_event(MockEvent::StreamOpened(peer_id.clone(), PROTOCOL))
            .expect("Should handle");

        test_data
            .service
            .send_to_random_authentication(message.clone(), iter::once(peer_id.clone()).collect());

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
        let mut test_data = TestData::prepare();

        let peer_id = random_peer_id();
        let other_peer_id = random_peer_id();

        let message = message(1);

        test_data
            .service
            .handle_network_event(MockEvent::StreamOpened(other_peer_id.clone(), PROTOCOL))
            .expect("Should handle");

        test_data
            .service
            .send_to_random_authentication(message.clone(), iter::once(peer_id.clone()).collect());

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
