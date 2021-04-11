use crate::{
    communication::{
        dummy_topic,
        gossip::{
            FetchRequest, FetchResponse, GossipMessage, GossipValidator, Multicast, PeerReport,
            SignedUnit,
        },
    },
    config::Config,
    hash::Hash,
    AuthorityId, AuthorityKeystore, UnitCoord,
};
use codec::{Decode, Encode};
use futures::{
    channel::{mpsc, mpsc::SendError},
    prelude::*,
    Future, FutureExt, StreamExt,
};

use log::{debug, error};
use parking_lot::Mutex;
use prometheus_endpoint::Registry;
use rush::{NotificationIn, NotificationOut};
use sc_network::{NetworkService, PeerId};
use sc_network_gossip::{GossipEngine, Network as GossipNetwork, TopicNotification};
use sp_runtime::traits::Block;
use sp_utils::mpsc::TracingUnboundedReceiver;
use std::{
    collections::{BinaryHeap, HashMap, HashSet},
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use std::cmp::Ordering;
use tokio::time;

pub const FETCH_INTERVAL: time::Duration = time::Duration::from_secs(4);
pub const TICK_INTERVAL: time::Duration = time::Duration::from_millis(100);

#[derive(Debug)]
enum ErrorKind {
    StartSendFail(SendError),
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        use ErrorKind::*;
        match self {
            StartSendFail(e) => write!(f, "failed to send on channel: {}", e),
        }
    }
}

impl Error for ErrorKind {}

#[derive(Debug)]
pub struct NetworkError(Box<ErrorKind>);

impl Display for NetworkError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(&self.0, f)
    }
}

impl Error for NetworkError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

impl From<ErrorKind> for NetworkError {
    fn from(e: ErrorKind) -> Self {
        NetworkError(Box::new(e))
    }
}

impl From<SendError> for NetworkError {
    fn from(e: SendError) -> Self {
        NetworkError(Box::new(ErrorKind::StartSendFail(e)))
    }
}

pub type NetworkResult<T> = Result<T, NetworkError>;

/// Name of the notifications protocol used by Aleph Zero. This is how messages
/// are subscribed to to ensure that we are gossiping and communicating with our
/// own network.
pub(crate) const ALEPH_PROTOCOL_NAME: &str = "/cardinals/aleph/1";

pub trait Network<B: Block>: GossipNetwork<B> + Clone + Send + Sync + 'static {}

impl<B: Block, H: Hash> Network<B> for Arc<NetworkService<B, H>> {}

// Just a wrapper around UnboundedSender -- not sure how to get rid of it.
// It makes the Error type compatible with the Environment trait in rush.
pub struct NotificationOutSender<B: Block, H: Hash> {
    tx: mpsc::UnboundedSender<NotificationOut<B::Hash, H>>,
}

impl<B: Block, H: Hash> Sink<NotificationOut<B::Hash, H>> for NotificationOutSender<B, H> {
    type Error = NetworkError;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(
        mut self: Pin<&mut Self>,
        item: NotificationOut<B::Hash, H>,
    ) -> NetworkResult<()> {
        self.tx.start_send(item).map_err(|e| e.into())
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<NetworkResult<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<NetworkResult<()>> {
        Sink::poll_close(Pin::new(&mut self.tx), cx).map(|elem| elem.map_err(|e| e.into()))
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct ScheduledRequest {
    coord: UnitCoord,
    scheduled_time: time::Instant,
}

impl ScheduledRequest {
    fn new(coord: UnitCoord, scheduled_time: time::Instant) -> Self {
        ScheduledRequest {
            coord,
            scheduled_time,
        }
    }
}

impl Ord for ScheduledRequest {
    fn cmp(&self, other: &Self) -> Ordering {
        // we want earlier times to come first when used in max-heap, hence the below:
        other.scheduled_time.cmp(&self.scheduled_time)
    }
}

impl PartialOrd for ScheduledRequest {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub(crate) struct UnitStore<B: Block, H: Hash> {
    by_coord: HashMap<UnitCoord, SignedUnit<B, H>>,
    by_hash: HashSet<H>,
}

impl<B: Block, H: Hash> UnitStore<B, H> {
    pub(crate) fn new() -> Self {
        UnitStore {
            by_coord: HashMap::new(),
            by_hash: HashSet::new(),
        }
    }

    pub(crate) fn unit_by_coord(&self, coord: UnitCoord) -> Option<&SignedUnit<B, H>> {
        self.by_coord.get(&coord)
    }

    pub(crate) fn contains_hash(&self, hash: &H) -> bool {
        self.by_hash.contains(hash)
    }

    pub(crate) fn contains_coord(&self, coord: &UnitCoord) -> bool {
        self.by_coord.contains_key(coord)
    }

    pub(crate) fn add_unit(&mut self, su: SignedUnit<B, H>) {
        self.by_hash.insert(su.unit.hash());
        let coord = (&su.unit).into();
        self.by_coord.insert(coord, su);
    }
}

#[derive(Clone)]
pub(crate) struct NetworkBridge<B: Block, H: Hash, N: Network<B>> {
    _network_service: N,
    gossip_engine: Arc<Mutex<GossipEngine<B>>>,
    gossip_validator: Arc<GossipValidator<B, H>>,
    peer_report_handle: Arc<Mutex<TracingUnboundedReceiver<PeerReport>>>,
    rx_consensus: Arc<Mutex<Option<mpsc::UnboundedReceiver<NotificationOut<B::Hash, H>>>>>,
    tx_consensus: Arc<Mutex<Option<mpsc::UnboundedSender<NotificationIn<B::Hash, H>>>>>,
    rx_network: Arc<Mutex<mpsc::Receiver<TopicNotification>>>,
    auth_cryptostore: AuthorityKeystore,
    // TODO: one can try optimizing: instead of Mutex use RwLocks on internal structures inside of UnitStore
    store: Arc<Mutex<UnitStore<B, H>>>,
    requests: Arc<Mutex<BinaryHeap<ScheduledRequest>>>,
    request_ticker: Arc<Mutex<time::Interval>>,
}

impl<B: Block, H: Hash, N: Network<B>> NetworkBridge<B, H, N> {
    pub(crate) fn new(
        network_service: N,
        _config: Option<Config>,
        registry: Option<&Registry>,
        authorities: Vec<AuthorityId>,
        auth_cryptostore: AuthorityKeystore,
    ) -> Self {
        let (gossip_validator, peer_report_handle) = {
            let (validator, peer_report_handle) = GossipValidator::<B, H>::new(registry);
            let validator = Arc::new(validator);
            let peer_report_handle = Arc::new(Mutex::new(peer_report_handle));
            (validator, peer_report_handle)
        };
        let gossip_engine = Arc::new(Mutex::new(GossipEngine::new(
            network_service.clone(),
            ALEPH_PROTOCOL_NAME,
            gossip_validator.clone(),
            None,
        )));
        gossip_validator.set_authorities(authorities);

        let topic = dummy_topic::<B>();
        let rx_network = Arc::new(Mutex::new(gossip_engine.lock().messages_for(topic)));

        NetworkBridge {
            _network_service: network_service,
            gossip_engine,
            gossip_validator,
            peer_report_handle,
            rx_consensus: Arc::new(Mutex::new(None)),
            tx_consensus: Arc::new(Mutex::new(None)),
            rx_network,
            auth_cryptostore,
            store: Arc::new(Mutex::new(UnitStore::new())),
            requests: Arc::new(Mutex::new(BinaryHeap::new())),
            request_ticker: Arc::new(Mutex::new(time::interval(TICK_INTERVAL))),
        }
    }

    pub(crate) fn note_pending_fetch_request(&self, peer: PeerId, coord: UnitCoord) {
        self.gossip_validator
            .note_pending_fetch_request(peer, coord)
    }

    pub(crate) fn communication(
        &self,
    ) -> (
        NotificationOutSender<B, H>,
        mpsc::UnboundedReceiver<NotificationIn<B::Hash, H>>,
    ) {
        let (tx_out, rx_out) = mpsc::unbounded();
        // NOTE: can this be done without mutexes? How?
        self.rx_consensus.lock().replace(rx_out);
        // NOTE: it should be possible to get rid of NotificationOutSender -- this is only a wrapper around
        // a channel endpoint with a suitable Error type (required by rush).
        let tx_out = NotificationOutSender::<B, H> { tx: tx_out };
        let (tx_in, rx_in) = mpsc::unbounded();
        self.tx_consensus.lock().replace(tx_in);
        (tx_out, rx_in)
    }

    fn send_consensus_notification(&self, notification: NotificationIn<B::Hash, H>) {
        if let Err(e) = self
            .tx_consensus
            .lock()
            .as_ref()
            .expect("Channel to consensus must be open.")
            .unbounded_send(notification)
        {
            debug!(target: "afa", "Error when sending notification {:?}.", e);
        }
    }

    fn on_create_notification(&self, u: rush::Unit<B::Hash, H>) {
        let signed_unit = super::gossip::sign_unit::<B, H>(&self.auth_cryptostore, u);
        let message = GossipMessage::Multicast(Multicast {
            signed_unit: signed_unit.clone(),
        });
        self.store.lock().add_unit(signed_unit);

        let topic: <B as Block>::Hash = dummy_topic::<B>();
        debug!(target: "afa", "Sending a unit over network.");
        self.gossip_engine
            .lock()
            .gossip_message(topic, message.encode(), false);
    }

    // Pulls requests from the priority queue (sorted by scheduled time) and sends them to random peers
    // as long as they are scheduled at time <= curr_time
    pub(crate) fn trigger_requests(&self) {
        loop {
            let curr_time = time::Instant::now();
            let maybe_coord = {
                let mut requests = self.requests.lock();
                if requests.is_empty() || requests.peek().unwrap().scheduled_time > curr_time {
                    None
                } else {
                    Some(requests.pop().unwrap().coord)
                }
            };

            if let Some(coord) = maybe_coord {
                debug!(target: "afa", "Starting request for {:?}", coord);
                // If we already have a unit with such a coord in our store then there is no need to request it.
                // It will be sent to consensus soon (or have already been sent).
                if !self.store.lock().contains_coord(&coord) {
                    let maybe_peer_id = self.gossip_validator.get_random_peer();
                    if let Some(peer_id) = maybe_peer_id {
                        let message =
                            GossipMessage::<B, H>::FetchRequest(FetchRequest { coord }).encode();
                        self.gossip_engine
                            .lock()
                            .send_message(vec![peer_id], message);
                        self.note_pending_fetch_request(peer_id, coord);
                        debug!(target: "afa", "Fetch request sent {:?} to peer {:?}.", coord, peer_id);
                    } else {
                        debug!(target: "afa", "Trying to request {:?} but no peer is available.", coord);
                    }
                    // Schedule a new request in case this one gets no answer.
                    self.requests
                        .lock()
                        .push(ScheduledRequest::new(coord, curr_time + FETCH_INTERVAL));
                } else {
                    debug!(target: "afa", "Request dropped as the unit is in store already {:?}", coord);
                }
            } else {
                break;
            }
        }
    }

    pub(crate) fn on_missing_notification(&self, coords: Vec<UnitCoord>) {
        debug!(target: "afa", "Dealing with missing notification {:?}.", coords);
        let curr_time = time::Instant::now();
        for coord in coords {
            if !self.store.lock().contains_coord(&coord) {
                self.requests
                    .lock()
                    .push(ScheduledRequest::new(coord, curr_time));
            }
        }
        self.trigger_requests();
    }

    fn on_consensus_notification(&self, notification: NotificationOut<B::Hash, H>) {
        match notification {
            NotificationOut::CreatedUnit(u) => {
                self.on_create_notification(u);
            }
            NotificationOut::MissingUnits(coords, _aux) => {
                let n_coords = {
                    let mut n_coords: Vec<UnitCoord> = Vec::with_capacity(coords.len());
                    for coord in coords {
                        n_coords.push(coord.into());
                    }
                    n_coords
                };
                self.on_missing_notification(n_coords);
            }
        }
    }

    fn on_unit_received(&self, su: SignedUnit<B, H>) {
        let mut store = self.store.lock();
        if !store.contains_hash(&su.unit.hash()) {
            self.send_consensus_notification(NotificationIn::NewUnits(vec![su.unit.clone()]));
            store.add_unit(su);
        }
    }

    fn on_fetch_request(&self, peer_id: PeerId, coord: UnitCoord) {
        debug!(target: "afa", "Received fetch request for coord {:?} from {:?}.", coord, peer_id);
        let maybe_su = (self.store.lock().unit_by_coord(coord)).cloned();

        if let Some(su) = maybe_su {
            debug!(target: "afa", "Answering fetch request for coord {:?} from {:?}.", coord, peer_id);
            let message =
                GossipMessage::<B, H>::FetchResponse(FetchResponse { signed_unit: su }).encode();
            self.gossip_engine
                .lock()
                .send_message(vec![peer_id], message);
        }
    }

    fn on_network_message(&self, notification: TopicNotification) {
        let who = notification.sender;
        let decoded = GossipMessage::<B, H>::decode(&mut &notification.message[..]);
        match decoded {
            Ok(message) => match message {
                GossipMessage::Multicast(m) => {
                    self.on_unit_received(m.signed_unit);
                }
                GossipMessage::FetchRequest(m) => {
                    if let Some(peer_id) = who {
                        self.on_fetch_request(peer_id, m.coord);
                    } else {
                        error!(target: "afa", "Fetch request from unknown peer {:?}.", m);
                    }
                }
                GossipMessage::FetchResponse(m) => {
                    debug!(target: "afa", "Fetch response received {:?}.", m);
                    self.on_unit_received(m.signed_unit);
                }
            },
            Err(e) => {
                error!(target: "afa", "Error in decoding a message in network bridge {:?}.", e);
            }
        }
    }
}

impl<B: Block, H: Hash, N: Network<B>> Future for NetworkBridge<B, H, N> {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            match self.peer_report_handle.lock().poll_next_unpin(cx) {
                Poll::Ready(Some(PeerReport { who, change })) => {
                    self.gossip_engine.lock().report(who, change);
                }
                Poll::Ready(None) => {
                    debug!(target: "afa", "Gossip validator report stream closed.");
                    break;
                }
                Poll::Pending => break,
            }
        }

        loop {
            let mut maybe_rx = self.rx_consensus.lock();
            if maybe_rx.is_some() {
                match maybe_rx.as_mut().unwrap().poll_next_unpin(cx) {
                    Poll::Ready(Some(notification)) => {
                        self.on_consensus_notification(notification);
                    }
                    Poll::Ready(None) => {
                        error!(target: "afa", "Consensus notification stream closed.");
                        return Poll::Ready(());
                    }
                    Poll::Pending => break,
                }
            }
        }

        loop {
            let mut rx = self.rx_network.lock();
            match rx.poll_next_unpin(cx) {
                Poll::Ready(Some(message)) => {
                    self.on_network_message(message);
                }
                Poll::Ready(None) => {
                    error!(target: "afa", "Network message stream closed.");
                    return Poll::Ready(());
                }
                Poll::Pending => break,
            }
        }

        // this is to make sure requests are triggered frequently
        while self.request_ticker.lock().poll_next_unpin(cx).is_ready() {
            self.trigger_requests();
        }
        self.gossip_engine.lock().poll_unpin(cx).map(|_| {
            debug!(target: "afa", "Gossip engine future finished");
        })
    }
}
