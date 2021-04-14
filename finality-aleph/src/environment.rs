use log::{debug, error};
use sc_client_api::backend::Backend;
use sp_consensus::SelectChain;
use sp_runtime::{
    generic::BlockId,
    traits::{Block, Header},
};
use std::{marker::PhantomData, sync::Arc};

use crate::communication::gossip;

use crate::{
    communication::{
        dummy_topic,
        gossip::{
            FetchRequest, FetchResponse, FullUnit, GossipMessage, GossipValidator, Multicast,
            PeerReport, SignedUnit,
        },
    },
    config::Config,
    hash::Hash,
    AuthorityId, AuthorityKeystore,
};
use codec::{Decode, Encode};
use futures::{channel::mpsc, prelude::*, Future, FutureExt, StreamExt};

use prometheus_endpoint::Registry;
use rush::{NotificationIn, NotificationOut, UnitCoord, Unit};
use sc_network::{NetworkService, PeerId};
use sc_network_gossip::{GossipEngine, Network as GossipNetwork, TopicNotification};
use sp_utils::mpsc::TracingUnboundedReceiver;
use std::{
    collections::{BinaryHeap, HashMap},
    pin::Pin,
    task::{Context, Poll},
};

use std::cmp::Ordering;
use tokio::time;


pub const FETCH_INTERVAL: time::Duration = time::Duration::from_secs(4);
pub const TICK_INTERVAL: time::Duration = time::Duration::from_millis(100);

/// Name of the network protocol used by Aleph Zero. This is how messages
/// are subscribed to to ensure that we are gossiping and communicating with our
/// own network.
pub(crate) const ALEPH_PROTOCOL_NAME: &str = "/cardinals/aleph/1";

pub trait Network<B: Block>: GossipNetwork<B> + Clone + Send + Sync + 'static {}

impl<B: Block, H: Hash> Network<B> for Arc<NetworkService<B, H>> {}

// Just a wrapper around UnboundedSender -- not sure how to get rid of it.
// It makes the Error type compatible with the Environment trait in rush.
pub struct NotificationOutSender<H: Hash> {
    tx: mpsc::UnboundedSender<NotificationOut<H>>,
}

impl<H: Hash> Sink<NotificationOut<H>> for NotificationOutSender<H> {
    type Error = Box<dyn std::error::Error>;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(mut self: Pin<&mut Self>, item: NotificationOut<H>) -> Result<(), Self::Error> {
        self.tx.start_send(item).map_err(|e| e.into())
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
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
    by_hash: HashMap<H, SignedUnit<B, H>>,
}

impl<B: Block, H: Hash> UnitStore<B, H> {
    pub(crate) fn new() -> Self {
        UnitStore {
            by_coord: HashMap::new(),
            by_hash: HashMap::new(),
        }
    }

    pub(crate) fn unit_by_coord(&self, coord: UnitCoord) -> Option<&SignedUnit<B, H>> {
        self.by_coord.get(&coord)
    }

    pub(crate) fn unit_by_hash(&self, hash: &H) -> Option<&SignedUnit<B, H>> {
        self.by_hash.get(hash)
    }

    pub(crate) fn contains_hash(&self, hash: &H) -> bool {
        self.by_hash.contains_key(hash)
    }

    pub(crate) fn contains_coord(&self, coord: &UnitCoord) -> bool {
        self.by_coord.contains_key(coord)
    }

    pub(crate) fn add_unit(&mut self, hash: H, su: SignedUnit<B, H>) {
        self.by_hash.insert(hash, su.clone());
        let coord = (su.unit.inner.round(), su.unit.inner.creator()).into();
        self.by_coord.insert(coord, su);
    }
}

pub(crate) struct Environment<B: Block, H: Hash, N: Network<B>, C, BE, SC> {
    _network_service: N,
    client: Arc<C>,
    select_chain: SC,
    gossip_engine: GossipEngine<B>,
    gossip_validator: Arc<GossipValidator<B, H>>,
    peer_report_handle: TracingUnboundedReceiver<PeerReport>,
    rx_consensus: Option<mpsc::UnboundedReceiver<NotificationOut<H>>>,
    tx_consensus: Option<mpsc::UnboundedSender<NotificationIn<H>>>,
    rx_network: futures::channel::mpsc::Receiver<TopicNotification>,
    rx_order: Option<tokio::sync::mpsc::UnboundedReceiver<Vec<H>>>,
    auth_cryptostore: AuthorityKeystore,
    store: UnitStore<B, H>,
    requests: BinaryHeap<ScheduledRequest>,
    request_ticker: time::Interval,
    hashing: Box<dyn Fn(&[u8]) -> H + Send>,
    _phantom: std::marker::PhantomData<(B, BE)>,
}

impl<B, H, N: Network<B>, C, BE, SC> Environment<B, H, N, C, BE, SC>
where
    B: Block,
    H: Hash,
    N: Network<B>,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
{
    pub(crate) fn new(
        client: Arc<C>,
        network_service: N,
        select_chain: SC,
        _config: Option<Config>,
        registry: Option<&Registry>,
        authorities: Vec<AuthorityId>,
        auth_cryptostore: AuthorityKeystore,
        hashing: impl Fn(&[u8]) -> H + Send + Copy + 'static,
    ) -> Self {
        let (gossip_validator, peer_report_handle) = {
            let (validator, peer_report_handle) = GossipValidator::<B, H>::new(registry);
            let validator = Arc::new(validator);
            (validator, peer_report_handle)
        };
        let mut gossip_engine = GossipEngine::new(
            network_service.clone(),
            ALEPH_PROTOCOL_NAME,
            gossip_validator.clone(),
            None,
        );
        gossip_validator.set_authorities(authorities);

        let topic = dummy_topic::<B>();
        let rx_network = gossip_engine.messages_for(topic);

        Environment {
            _network_service: network_service,
            client,
            select_chain,
            gossip_engine,
            gossip_validator,
            peer_report_handle,
            rx_consensus: None,
            tx_consensus: None,
            rx_network,
            rx_order: None,
            auth_cryptostore,
            store: UnitStore::new(),
            requests: BinaryHeap::new(),
            request_ticker: time::interval(TICK_INTERVAL),
            hashing: Box::new(hashing),
            _phantom: PhantomData,
        }
    }

    pub(crate) fn note_pending_fetch_request(&self, peer: PeerId, coord: UnitCoord) {
        self.gossip_validator
            .note_pending_fetch_request(peer, coord)
    }

    pub(crate) fn consensus_data(
        &mut self,
    ) -> (
        NotificationOutSender<H>,
        mpsc::UnboundedReceiver<NotificationIn<H>>,
        tokio::sync::mpsc::UnboundedSender<Vec<H>>,
    ) {
        let (tx_out, rx_out) = mpsc::unbounded();
        self.rx_consensus.replace(rx_out);
        let tx_out = NotificationOutSender::<H> { tx: tx_out };
        let (tx_in, rx_in) = mpsc::unbounded();
        self.tx_consensus.replace(tx_in);
        let (tx_order, rx_order) = tokio::sync::mpsc::unbounded_channel();
        self.rx_order.replace(rx_order);
        (tx_out, rx_in, tx_order)
    }

    fn send_consensus_notification(&mut self, notification: NotificationIn<H>) {
        if let Err(e) = self
            .tx_consensus
            .as_mut()
            .expect("Channel to consensus must be open.")
            .start_send(notification)
        {
            debug!(target: "env", "Error when sending notification {:?}.", e);
        }
    }

    fn on_create_notification(&mut self, u: rush::PreUnit<H>) {
        let block_hash = self.best_block();
        let full_unit = FullUnit {
            inner: u,
            block_hash,
        };
        let signed_unit = gossip::sign_unit::<B, H>(&self.auth_cryptostore, full_unit);
        let message = GossipMessage::Multicast(Multicast {
            signed_unit: signed_unit.clone(),
        });

        let hash = signed_unit.hash(&self.hashing);
        // We also need to pass this unit to our instance of Consensus.
        let unit = Unit::new_from_preunit(signed_unit.unit.inner.clone(), hash);
        self.send_consensus_notification(NotificationIn::NewUnits(vec![unit]));

        self.store.add_unit(hash, signed_unit);

        let topic: <B as Block>::Hash = dummy_topic::<B>();
        debug!(target: "env", "Sending a unit over network.");
        self.gossip_engine
            .gossip_message(topic, message.encode(), false);
    }

    // Pulls requests from the priority queue (sorted by scheduled time) and sends them to random peers
    // as long as they are scheduled at time <= curr_time
    pub(crate) fn trigger_requests(&mut self) {
        loop {
            let curr_time = time::Instant::now();
            let maybe_coord = {
                if self.requests.is_empty()
                    || self.requests.peek().unwrap().scheduled_time > curr_time
                {
                    None
                } else {
                    Some(self.requests.pop().unwrap().coord)
                }
            };

            if let Some(coord) = maybe_coord {
                debug!(target: "env", "Starting request for {:?}", coord);
                // If we already have a unit with such a coord in our store then there is no need to request it.
                // It will be sent to consensus soon (or have already been sent).
                if !self.store.contains_coord(&coord) {
                    let maybe_peer_id = self.gossip_validator.get_random_peer();
                    if let Some(peer_id) = maybe_peer_id {
                        let message =
                            GossipMessage::<B, H>::FetchRequest(FetchRequest { coord }).encode();
                        self.gossip_engine.send_message(vec![peer_id], message);
                        self.note_pending_fetch_request(peer_id, coord);
                        debug!(target: "env", "Fetch request sent {:?} to peer {:?}.", coord, peer_id);
                    } else {
                        debug!(target: "env", "Trying to request {:?} but no peer is available.", coord);
                    }
                    // Schedule a new request in case this one gets no answer.
                    self.requests
                        .push(ScheduledRequest::new(coord, curr_time + FETCH_INTERVAL));
                } else {
                    debug!(target: "env", "Request dropped as the unit is in store already {:?}", coord);
                }
            } else {
                break;
            }
        }
    }

    pub(crate) fn on_missing_notification(&mut self, coords: Vec<UnitCoord>) {
        debug!(target: "env", "Dealing with missing notification {:?}.", coords);
        let curr_time = time::Instant::now();
        for coord in coords {
            if !self.store.contains_coord(&coord) {
                self.requests.push(ScheduledRequest::new(coord, curr_time));
            }
        }
        self.trigger_requests();
    }

    fn on_consensus_notification(&mut self, notification: NotificationOut<H>) {
        match notification {
            NotificationOut::CreatedPreUnit(pu) => {
                self.on_create_notification(pu);
            }
            NotificationOut::MissingUnits(coords, _aux) => {
                self.on_missing_notification(coords);
            }
        }
    }

    fn on_unit_received(&mut self, su: SignedUnit<B, H>) {
        let hash = su.hash(&self.hashing);
        if !self.store.contains_hash(&hash) {
            let unit = Unit::new_from_preunit(su.unit.inner.clone(), hash);
            self.send_consensus_notification(NotificationIn::NewUnits(vec![unit]));
            self.store.add_unit(hash, su);
        }
    }

    fn on_fetch_request(&mut self, peer_id: PeerId, coord: UnitCoord) {
        debug!(target: "env", "Received fetch request for coord {:?} from {:?}.", coord, peer_id);
        let maybe_su = (self.store.unit_by_coord(coord)).cloned();

        if let Some(su) = maybe_su {
            debug!(target: "env", "Answering fetch request for coord {:?} from {:?}.", coord, peer_id);
            let message =
                GossipMessage::<B, H>::FetchResponse(FetchResponse { signed_unit: su }).encode();
            self.gossip_engine.send_message(vec![peer_id], message);
        }
    }

    fn on_ordered_batch(&mut self, batch: Vec<H>) {
        for h in batch {
            let u = self
                .store
                .unit_by_hash(&h)
                .expect("Ordered units must be in store");
            let block_hash = u.unit.block_hash;
            if self.check_extends_finalized(block_hash) {
                self.finalize_block(block_hash);
                debug!(target: "env", "Finalized block hash {}.", block_hash);
            }
        }
    }

    fn on_network_message(&mut self, notification: TopicNotification) {
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
                        error!(target: "env", "Fetch request from unknown peer {:?}.", m);
                    }
                }
                GossipMessage::FetchResponse(m) => {
                    debug!(target: "env", "Fetch response received {:?}.", m);
                    self.on_unit_received(m.signed_unit);
                }
            },
            Err(e) => {
                error!(target: "env", "Error in decoding a message in network bridge {:?}.", e);
            }
        }
    }

    fn best_block(&self) -> B::Hash {
        self.select_chain
            .best_chain()
            .expect("No best chain")
            .hash()
    }

    fn check_extends_finalized(&self, h: B::Hash) -> bool {
        let head_finalized = self.client.info().finalized_hash;
        if h == head_finalized {
            return false;
        }
        let lca = sp_blockchain::lowest_common_ancestor(self.client.as_ref(), h, head_finalized)
            .expect("No lowest common ancestor");
        lca.hash == head_finalized
    }

    fn finalize_block(&self, h: B::Hash) {
        finalize_block(self.client.clone(), h);
    }
}

impl<B, H, N, C, BE, SC> Unpin for Environment<B, H, N, C, BE, SC>
where
    B: Block,
    H: Hash,
    N: Network<B>,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
{
}

impl<B: Block, H: Hash, N: Network<B>, C, BE, SC> Future for Environment<B, H, N, C, BE, SC>
where
    B: Block,
    H: Hash,
    N: Network<B>,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
{
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            match self.peer_report_handle.poll_next_unpin(cx) {
                Poll::Ready(Some(PeerReport { who, change })) => {
                    self.gossip_engine.report(who, change);
                }
                Poll::Ready(None) => {
                    debug!(target: "env", "Gossip validator report stream closed.");
                    break;
                }
                Poll::Pending => break,
            }
        }

        if self.rx_consensus.is_some() {
            loop {
                match self.rx_consensus.as_mut().unwrap().poll_next_unpin(cx) {
                    Poll::Ready(Some(notification)) => {
                        self.on_consensus_notification(notification);
                    }
                    Poll::Ready(None) => {
                        error!(target: "env", "Consensus notification stream closed.");
                        return Poll::Ready(());
                    }
                    Poll::Pending => break,
                }
            }
        }

        // NOTE: the loop below finalizes blocks based on the ordered units received from consensus,
        // for efficiency it might be necessary to run this in a separate thread, although then self.store
        // would need to be synchronized (or perhaps a copy could be stored in the finalizer thread).
        if self.rx_order.is_some() {
            loop {
                match self.rx_order.as_mut().unwrap().poll_next_unpin(cx) {
                    Poll::Ready(Some(batch)) => {
                        self.on_ordered_batch(batch);
                    }
                    Poll::Ready(None) => {
                        error!(target: "env", "Consensus order stream closed.");
                        return Poll::Ready(());
                    }
                    Poll::Pending => break,
                }
            }
        }

        loop {
            match self.rx_network.poll_next_unpin(cx) {
                Poll::Ready(Some(message)) => {
                    self.on_network_message(message);
                }
                Poll::Ready(None) => {
                    error!(target: "env", "Network message stream closed.");
                    return Poll::Ready(());
                }
                Poll::Pending => break,
            }
        }

        // this is to make sure requests are triggered frequently
        while self.request_ticker.poll_next_unpin(cx).is_ready() {
            self.trigger_requests();
        }

        self.gossip_engine.poll_unpin(cx).map(|_| {
            debug!(target: "env", "Gossip engine future finished");
        })
    }
}

pub(crate) fn finalize_block<BE, B, C>(client: Arc<C>, hash: B::Hash)
where
    B: Block,
    BE: Backend<B>,
    C: crate::ClientForAleph<B, BE>,
{
    let block_number = match client.number(hash) {
        Ok(Some(number)) => number,
        _ => {
            error!(target: "env", "a block with hash {} should already be in chain", hash);
            return;
        }
    };
    let info = client.info();

    if info.finalized_number >= block_number {
        error!(target: "env", "trying to finalized a block with hash {} and number {}
               that is not greater than already finalized {}", hash, block_number, info.finalized_number);
        return;
    }

    let status = client.info();
    debug!(target: "env", "Finalizing block with hash {:?}. Previous best: #{:?}.", hash, status.finalized_number);

    let _update_res = client.lock_import_and_run(|import_op| {
        // NOTE: all other finalization logic should come here, inside the lock
        client.apply_finality(import_op, BlockId::Hash(hash), None, true)
    });

    let status = client.info();
    debug!(target: "env", "Finalized block with hash {:?}. Current best: #{:?}.", hash,status.finalized_number);
}
