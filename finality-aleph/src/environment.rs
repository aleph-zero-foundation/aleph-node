use log::{debug, error};
use sc_client_api::backend::Backend;
use sp_consensus::SelectChain;
use sp_runtime::{
    generic::BlockId,
    traits::{Block, Header},
};
use std::{marker::PhantomData, sync::Arc};

use crate::{
    config::Config,
    hash::Hash,
    messages::{sign_unit, ConsensusMessage, FullUnit, NetworkMessage, SignedUnit},
    network::{NetworkCommand, NetworkEvent},
    AuthorityId, AuthorityKeystore, EpochId,
};
use futures::{channel::mpsc, prelude::*, Future, StreamExt};

use rush::{NotificationIn, NotificationOut, Unit, UnitCoord};
use sc_network::PeerId;
use std::{
    collections::{BinaryHeap, HashMap},
    pin::Pin,
    task::{Context, Poll},
};

use std::cmp::Ordering;
use tokio::time;

pub const FETCH_INTERVAL: time::Duration = time::Duration::from_secs(4);
pub const TICK_INTERVAL: time::Duration = time::Duration::from_millis(100);
pub const INITIAL_MULTICAST_DELAY: time::Duration = time::Duration::from_secs(10);

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

#[derive(Clone, Debug, Eq, PartialEq)]
enum Task<H: Hash> {
    CoordRequest(UnitCoord),
    // the hash of a unit, and the delay before repeating the multicast
    UnitMulticast(H, time::Duration),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ScheduledTask<H: Hash> {
    task: Task<H>,
    scheduled_time: time::Instant,
}

impl<H: Hash> ScheduledTask<H> {
    fn new(task: Task<H>, scheduled_time: time::Instant) -> Self {
        ScheduledTask {
            task,
            scheduled_time,
        }
    }
}

impl<H: Hash> Ord for ScheduledTask<H> {
    fn cmp(&self, other: &Self) -> Ordering {
        // we want earlier times to come first when used in max-heap, hence the below:
        other.scheduled_time.cmp(&self.scheduled_time)
    }
}

impl<H: Hash> PartialOrd for ScheduledTask<H> {
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

pub(crate) struct Environment<B: Block, H: Hash, C, BE, SC> {
    client: Arc<C>,
    select_chain: SC,
    tx_consensus: Option<mpsc::UnboundedSender<NotificationIn<H>>>,
    rx_consensus: Option<mpsc::UnboundedReceiver<NotificationOut<H>>>,
    tx_network: mpsc::UnboundedSender<NetworkCommand<B, H>>,
    rx_network: mpsc::UnboundedReceiver<NetworkEvent<B, H>>,
    rx_order: Option<tokio::sync::mpsc::UnboundedReceiver<Vec<H>>>,
    auth_cryptostore: AuthorityKeystore,
    store: UnitStore<B, H>,
    requests: BinaryHeap<ScheduledTask<H>>,
    request_ticker: time::Interval,
    hashing: Box<dyn Fn(&[u8]) -> H + Send>,
    epoch_id: EpochId,
    _phantom: std::marker::PhantomData<(B, BE)>,
}

impl<B, H, C, BE, SC> Environment<B, H, C, BE, SC>
where
    B: Block,
    H: Hash,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
{
    pub(crate) fn new(
        client: Arc<C>,
        select_chain: SC,
        tx_network: mpsc::UnboundedSender<NetworkCommand<B, H>>,
        rx_network: mpsc::UnboundedReceiver<NetworkEvent<B, H>>,
        _config: Option<Config>,
        _authorities: Vec<AuthorityId>,
        auth_cryptostore: AuthorityKeystore,
        hashing: impl Fn(&[u8]) -> H + Send + Copy + 'static,
        epoch_id: EpochId,
    ) -> Self {
        Environment {
            client,
            select_chain,
            tx_consensus: None,
            rx_consensus: None,
            tx_network,
            rx_network,
            rx_order: None,
            auth_cryptostore,
            store: UnitStore::new(),
            requests: BinaryHeap::new(),
            request_ticker: time::interval(TICK_INTERVAL),
            hashing: Box::new(hashing),
            epoch_id,
            _phantom: PhantomData,
        }
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

    fn form_network_message(&self, message: ConsensusMessage<B, H>) -> NetworkMessage<B, H> {
        NetworkMessage::Consensus(message, self.epoch_id)
    }

    fn on_create_notification(&mut self, u: rush::PreUnit<H>) {
        debug!(target: "env", "On create notification.");
        let block_hash = self.best_block();
        debug!(target: "env", "On create notification post best_block.");
        let full_unit = FullUnit {
            inner: u,
            block_hash,
        };
        //TODO: beware: sign_unit blocks and is quite slow!
        let signed_unit = sign_unit::<B, H>(&self.auth_cryptostore, full_unit);
        debug!(target: "env", "On create notification post sign_unit.");
        let hash = signed_unit.hash(&self.hashing);
        self.store.add_unit(hash, signed_unit.clone());
        // We also need to pass this unit to our instance of Consensus.
        let unit = Unit::new_from_preunit(signed_unit.unit.inner, hash);
        self.send_consensus_notification(NotificationIn::NewUnits(vec![unit]));
        let curr_time = time::Instant::now();
        let task = ScheduledTask::new(
            Task::UnitMulticast(hash, INITIAL_MULTICAST_DELAY),
            curr_time,
        );
        self.requests.push(task);
    }

    fn place_network_command(&self, command: NetworkCommand<B, H>) {
        if let Err(e) = self.tx_network.unbounded_send(command) {
            debug!(target: "env", "Failed to place network command {:?}.", e);
        }
    }

    // Pulls requests from the priority queue (sorted by scheduled time) and sends them to random peers
    // as long as they are scheduled at time <= curr_time
    pub(crate) fn trigger_tasks(&mut self) {
        loop {
            let curr_time = time::Instant::now();
            if self.requests.is_empty() || self.requests.peek().unwrap().scheduled_time > curr_time
            {
                break;
            }
            let task = self.requests.pop().expect("Queue must be non-empty").task;
            match task {
                Task::CoordRequest(coord) => {
                    debug!(target: "env", "Starting request for {:?}", coord);
                    // If we already have a unit with such a coord in our store then there is no need to request it.
                    // It will be sent to consensus soon (or have already been sent).
                    if !self.store.contains_coord(&coord) {
                        let message =
                            self.form_network_message(ConsensusMessage::FetchRequest(coord));
                        let command = NetworkCommand::SendToRandPeer(message);
                        self.place_network_command(command);
                        debug!(target: "env", "Fetch request for {:?} sent.", coord);
                        self.requests.push(ScheduledTask::new(
                            Task::CoordRequest(coord),
                            curr_time + FETCH_INTERVAL,
                        ));
                    } else {
                        debug!(target: "env", "Request dropped as the unit is in store already {:?}", coord);
                    }
                }
                Task::UnitMulticast(hash, interval) => {
                    let signed_unit = self
                        .store
                        .unit_by_hash(&hash)
                        .expect("Our units are in store.")
                        .clone();
                    let message =
                        self.form_network_message(ConsensusMessage::NewUnit(signed_unit.clone()));
                    debug!(target: "env", "Sending a unit {:?} over network after delay {:?}.", hash, interval);
                    let command = NetworkCommand::SendToAll(message);
                    if let Err(e) = self.tx_network.unbounded_send(command) {
                        debug!(target: "env", "Failed to place a multicast command in the network channel {:?}.", e);
                    }
                    //NOTE: we double the delay each time
                    self.requests.push(ScheduledTask::new(
                        Task::UnitMulticast(hash, interval * 2),
                        curr_time + interval,
                    ));
                }
            }
        }
    }

    pub(crate) fn on_missing_notification(&mut self, coords: Vec<UnitCoord>) {
        debug!(target: "env", "Dealing with missing notification {:?}.", coords);
        let curr_time = time::Instant::now();
        for coord in coords {
            if !self.store.contains_coord(&coord) {
                let task = ScheduledTask::new(Task::CoordRequest(coord), curr_time);
                self.requests.push(task);
            }
        }
        self.trigger_tasks();
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
        if su.verify_unit_signature() {
            let hash = su.hash(&self.hashing);
            if !self.store.contains_hash(&hash) {
                let unit = Unit::new_from_preunit(su.unit.inner.clone(), hash);
                self.send_consensus_notification(NotificationIn::NewUnits(vec![unit]));
                self.store.add_unit(hash, su);
            }
        } else {
            debug!("A unit with incorrect signature received! {:?}", su);
        }
    }

    fn on_fetch_request(&mut self, peer_id: PeerId, coord: UnitCoord) {
        debug!(target: "env", "Received fetch request for coord {:?} from {:?}.", coord, peer_id);
        let maybe_su = (self.store.unit_by_coord(coord)).cloned();

        if let Some(su) = maybe_su {
            debug!(target: "env", "Answering fetch request for coord {:?} from {:?}.", coord, peer_id);
            let message = self.form_network_message(ConsensusMessage::FetchResponse(su));
            let command = NetworkCommand::SendToPeer(message, peer_id);
            self.place_network_command(command);
        } else {
            debug!(target: "env", "Not answering fetch request for coord {:?}. Unit not in store.", coord);
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
    fn on_network_message(&mut self, message: ConsensusMessage<B, H>, sender: PeerId) {
        match message {
            ConsensusMessage::NewUnit(signed_unit) => {
                self.on_unit_received(signed_unit);
            }
            ConsensusMessage::FetchRequest(coord) => {
                self.on_fetch_request(sender, coord);
            }
            ConsensusMessage::FetchResponse(signed_unit) => {
                debug!(target: "env", "Fetch response received {:?}.", signed_unit);
                self.on_unit_received(signed_unit);
            }
        }
    }

    fn on_network_event(&mut self, event: NetworkEvent<B, H>) {
        match event {
            NetworkEvent::MessageReceived(message, sender) => {
                self.on_network_message(message, sender);
            }
            NetworkEvent::PeerConnected(peer_id) => {
                //TODO: might want to add support for this
                debug!("New peer connected: {:?}.", peer_id);
            }
            NetworkEvent::PeerDisconnected(peer_id) => {
                //TODO: might want to add support for this
                debug!("Peer disconnected {:?}.", peer_id);
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

impl<B, H, C, BE, SC> Unpin for Environment<B, H, C, BE, SC>
where
    B: Block,
    H: Hash,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
{
}

impl<B: Block, H: Hash, C, BE, SC> Future for Environment<B, H, C, BE, SC>
where
    B: Block,
    H: Hash,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
{
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        debug!(target: "env", "Polling environment.");
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
                Poll::Ready(Some(event)) => {
                    self.on_network_event(event);
                }
                Poll::Ready(None) => {
                    error!(target: "env", "Network event stream closed.");
                    return Poll::Ready(());
                }
                Poll::Pending => break,
            }
        }

        // this is to make sure requests are triggered frequently
        while self.request_ticker.poll_next_unpin(cx).is_ready() {
            self.trigger_tasks();
        }
        Poll::Pending
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
