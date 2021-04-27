use aleph_primitives::ALEPH_ENGINE_ID;
use codec::Encode;
use log::{debug, error};
use sc_client_api::backend::Backend;
use sp_consensus::SelectChain;
use sp_runtime::{
    generic::BlockId,
    traits::{Block, Header},
    Justification,
};
use std::{marker::PhantomData, sync::Arc};

use crate::{
    hash::Hash,
    messages::{sign_unit, ConsensusMessage, FullUnit, NetworkMessage, SignedUnit},
    network::{NetworkCommand, NetworkEvent},
    AuthorityId, AuthorityKeystore, EpochId,
};
use futures::{channel::mpsc, StreamExt};

use rush::{
    ControlHash, NodeCount, NodeIndex, NodeMap, NotificationIn, NotificationOut, Unit, UnitCoord,
};
use sc_network::PeerId;
use std::collections::{BinaryHeap, HashMap};

use crate::justification::AlephJustification;
use futures::stream::Fuse;
use sp_api::NumberFor;
use std::cmp::Ordering;
use tokio::time;

pub const FETCH_INTERVAL: time::Duration = time::Duration::from_secs(4);
pub const TICK_INTERVAL: time::Duration = time::Duration::from_millis(100);
pub const INITIAL_MULTICAST_DELAY: time::Duration = time::Duration::from_secs(10);

#[derive(Clone, Debug, Eq, PartialEq)]
enum Task<H: Hash> {
    CoordRequest(UnitCoord),
    ParentsRequest(H),
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
    parents: HashMap<H, Vec<H>>,
}

impl<B: Block, H: Hash> UnitStore<B, H> {
    pub(crate) fn new() -> Self {
        UnitStore {
            by_coord: HashMap::new(),
            by_hash: HashMap::new(),
            parents: HashMap::new(),
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

    pub(crate) fn add_parents(&mut self, hash: H, parents: Vec<H>) {
        self.parents.insert(hash, parents);
    }

    pub(crate) fn get_parents(&mut self, hash: H) -> Option<&Vec<H>> {
        self.parents.get(&hash)
    }
}

pub(crate) struct Environment<B: Block, H: Hash, C, BE, SC> {
    client: Arc<C>,
    select_chain: SC,
    tx_consensus: mpsc::UnboundedSender<NotificationIn<H>>,
    rx_consensus: mpsc::UnboundedReceiver<NotificationOut<H>>,
    tx_network: mpsc::UnboundedSender<NetworkCommand<B, H>>,
    rx_network: mpsc::UnboundedReceiver<NetworkEvent<B, H>>,
    rx_order: Fuse<tokio::sync::mpsc::UnboundedReceiver<Vec<H>>>,
    auth_cryptostore: AuthorityKeystore,
    store: UnitStore<B, H>,
    requests: BinaryHeap<ScheduledTask<H>>,
    hashing: Box<dyn Fn(&[u8]) -> H + Send>,
    epoch_id: EpochId,
    n_nodes: usize,
    _phantom: PhantomData<BE>,
}

impl<B, H, C, BE, SC> Environment<B, H, C, BE, SC>
where
    B: Block,
    H: Hash,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
{
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        client: Arc<C>,
        select_chain: SC,
        tx_consensus: mpsc::UnboundedSender<NotificationIn<H>>,
        rx_consensus: mpsc::UnboundedReceiver<NotificationOut<H>>,
        tx_network: mpsc::UnboundedSender<NetworkCommand<B, H>>,
        rx_network: mpsc::UnboundedReceiver<NetworkEvent<B, H>>,
        rx_order: tokio::sync::mpsc::UnboundedReceiver<Vec<H>>,
        authorities: Vec<AuthorityId>,
        auth_cryptostore: AuthorityKeystore,
        hashing: impl Fn(&[u8]) -> H + Send + Copy + 'static,
        epoch_id: EpochId,
    ) -> Self {
        Environment {
            client,
            select_chain,
            tx_consensus,
            rx_consensus,
            tx_network,
            rx_network,
            rx_order: rx_order.fuse(),
            auth_cryptostore,
            store: UnitStore::new(),
            requests: BinaryHeap::new(),
            hashing: Box::new(hashing),
            epoch_id,
            n_nodes: authorities.len(),
            _phantom: PhantomData,
        }
    }

    fn send_consensus_notification(&mut self, notification: NotificationIn<H>) {
        if let Err(e) = self.tx_consensus.unbounded_send(notification) {
            debug!(target: "env", "Error when sending notification {:?}.", e);
        }
    }

    fn form_network_message(&self, message: ConsensusMessage<B, H>) -> NetworkMessage<B, H> {
        NetworkMessage::Consensus(message, self.epoch_id)
    }

    fn on_create(&mut self, u: rush::PreUnit<H>) {
        debug!(target: "env", "On create notification.");
        let block_hash = self.best_block();
        debug!(target: "env", "On create notification post best_block.");
        let full_unit = FullUnit {
            inner: u,
            block_hash,
            epoch_id: self.epoch_id,
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
        while let Some(request) = self.requests.peek() {
            let curr_time = time::Instant::now();
            if request.scheduled_time > curr_time {
                break;
            }
            let request = self.requests.pop().expect("The element was peeked");
            let task = request.task;

            match task {
                Task::CoordRequest(coord) => {
                    self.schedule_coord_request(coord, curr_time);
                }
                Task::UnitMulticast(hash, interval) => {
                    self.schedule_unit_multicast(hash, interval, curr_time);
                }
                Task::ParentsRequest(u_hash) => {
                    self.schedule_parents_request(u_hash, curr_time);
                }
            }
        }
    }

    fn schedule_parents_request(&mut self, u_hash: H, curr_time: time::Instant) {
        if self.store.get_parents(u_hash).is_none() {
            let message = self.form_network_message(ConsensusMessage::RequestParents(u_hash));
            let command = NetworkCommand::SendToRandPeer(message);
            self.place_network_command(command);
            debug!(target: "env", "Fetch parents for {:} sent.", u_hash);
            self.requests.push(ScheduledTask::new(
                Task::ParentsRequest(u_hash),
                curr_time + FETCH_INTERVAL,
            ));
        } else {
            debug!(target: "env", "Request dropped as the parents are in store for {:}.", u_hash);
        }
    }

    fn schedule_coord_request(&mut self, coord: UnitCoord, curr_time: time::Instant) {
        debug!(target: "env", "Starting request for {:?}", coord);
        // If we already have a unit with such a coord in our store then there is no need to request it.
        // It will be sent to consensus soon (or have already been sent).
        if self.store.contains_coord(&coord) {
            debug!(target: "env", "Request dropped as the unit is in store already {:?}", coord);
            return;
        }
        let message = self.form_network_message(ConsensusMessage::RequestCoord(coord));
        let command = NetworkCommand::SendToRandPeer(message);
        self.place_network_command(command);
        debug!(target: "env", "Fetch request for {:?} sent.", coord);
        self.requests.push(ScheduledTask::new(
            Task::CoordRequest(coord),
            curr_time + FETCH_INTERVAL,
        ));
    }

    fn schedule_unit_multicast(
        &mut self,
        hash: H,
        interval: time::Duration,
        curr_time: time::Instant,
    ) {
        let signed_unit = self
            .store
            .unit_by_hash(&hash)
            .expect("Our units are in store.")
            .clone();
        let message = self.form_network_message(ConsensusMessage::NewUnit(signed_unit));
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

    pub(crate) fn on_missing_coords(&mut self, coords: Vec<UnitCoord>) {
        debug!(target: "env", "Dealing with missing coords notification {:?}.", coords);
        let curr_time = time::Instant::now();
        for coord in coords {
            if !self.store.contains_coord(&coord) {
                let task = ScheduledTask::new(Task::CoordRequest(coord), curr_time);
                self.requests.push(task);
            }
        }
        self.trigger_tasks();
    }

    fn on_wrong_control_hash(&mut self, u_hash: H) {
        debug!(target: "env", "Dealing with wrong control hash notification {:?}.", u_hash);
        if let Some(p_hashes) = self.store.get_parents(u_hash) {
            // We have the parents by some strange reason (someone sent us parents
            // without us requesting them).
            let p_hashes = p_hashes.clone();
            debug!(target: "env", "We have the parents for {:?} even though we did not request them.", u_hash);
            self.send_consensus_notification(NotificationIn::UnitParents(u_hash, p_hashes));
        } else {
            let curr_time = time::Instant::now();
            let task = ScheduledTask::new(Task::ParentsRequest(u_hash), curr_time);
            self.requests.push(task);
            self.trigger_tasks();
        }
    }

    fn on_consensus_notification(&mut self, notification: NotificationOut<H>) {
        match notification {
            NotificationOut::CreatedPreUnit(pu) => {
                self.on_create(pu);
            }
            NotificationOut::MissingUnits(coords, _aux) => {
                self.on_missing_coords(coords);
            }
            NotificationOut::WrongControlHash(h) => {
                self.on_wrong_control_hash(h);
            }
            NotificationOut::AddedToDag(h, p_hashes) => {
                //TODO: this is very RAM-heavy to store, optimizations needed
                self.store.add_parents(h, p_hashes);
            }
        }
    }

    /// Outputs the units hash in case it is correct.
    fn on_unit_received(&mut self, su: SignedUnit<B, H>) -> Option<H> {
        //TODO: make sure we check all that is necessary for unit correctness
        if su.unit.epoch_id != self.epoch_id {
            //NOTE: this implies malicious behavior as the unit's epoch_id
            // is incompatible with epoch_id of the message it arrived in.
            debug!("A unit with incorrect epoch_id received! {:?}", su);
            return None;
        }
        if su.verify_unit_signature() {
            let hash = su.hash(&self.hashing);
            if !self.store.contains_hash(&hash) {
                let unit = Unit::new_from_preunit(su.unit.inner.clone(), hash);
                self.send_consensus_notification(NotificationIn::NewUnits(vec![unit]));
                self.store.add_unit(hash, su);
            }
            Some(hash)
        } else {
            debug!("A unit with incorrect signature received! {:?}", su);
            None
        }
    }

    fn on_request_coord(&mut self, peer_id: PeerId, coord: UnitCoord) {
        debug!(target: "env", "Received fetch request for coord {:?} from {:?}.", coord, peer_id);
        let maybe_su = (self.store.unit_by_coord(coord)).cloned();

        if let Some(su) = maybe_su {
            debug!(target: "env", "Answering fetch request for coord {:?} from {:?}.", coord, peer_id);
            let message = self.form_network_message(ConsensusMessage::ResponseCoord(su));
            let command = NetworkCommand::SendToPeer(message, peer_id);
            self.place_network_command(command);
        } else {
            debug!(target: "env", "Not answering fetch request for coord {:?}. Unit not in store.", coord);
        }
    }

    fn on_request_parents(&mut self, peer_id: PeerId, u_hash: H) {
        debug!(target: "env", "Received parents request for hash {:?} from {:?}.", u_hash, peer_id);
        let maybe_p_hashes = self.store.get_parents(u_hash);

        if let Some(p_hashes) = maybe_p_hashes {
            let p_hashes = p_hashes.clone();
            debug!(target: "env", "Answering parents request for hash {:?} from {:?}.", u_hash, peer_id);
            let full_units = p_hashes
                .into_iter()
                .map(|hash| self.store.unit_by_hash(&hash).unwrap().clone())
                .collect();
            let message =
                self.form_network_message(ConsensusMessage::ResponseParents(u_hash, full_units));
            let command = NetworkCommand::SendToPeer(message, peer_id);
            self.place_network_command(command);
        } else {
            debug!(target: "env", "Not answering parents request for hash {:?}. Unit not in DAG yet.", u_hash);
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

    fn on_parents_response(&mut self, u_hash: H, parents: Vec<SignedUnit<B, H>>) {
        let maybe_u = self.store.unit_by_hash(&u_hash);
        if maybe_u.is_none() {
            debug!(target: "env", "We got parents but don't even know the unit. Ignoring.");
            return;
        }
        let u = maybe_u.unwrap();
        let u_round = u.unit.inner.round();
        let u_chash = u.unit.inner.control_hash.hash;
        let parent_ids: Vec<NodeIndex> = u
            .unit
            .inner
            .control_hash
            .parents
            .enumerate()
            .filter_map(|(i, b)| if *b { Some(i) } else { None })
            .collect();

        if parent_ids.len() != parents.len() {
            debug!(target: "env", "In received parent response expected {} parents got {} for unit {:?}.", parents.len(), parent_ids.len(), u_hash);
        }

        let mut p_hashes_node_map: NodeMap<Option<H>> =
            NodeMap::new_with_len(NodeCount(self.n_nodes));
        for (i, su) in parents.into_iter().enumerate() {
            if su.unit.inner.round() + 1 != u_round {
                debug!(target: "env", "In received parent response received a unit with wrong round.");
                return;
            }
            if su.unit.inner.creator() != parent_ids[i] {
                debug!(target: "env", "In received parent response received a unit with wrong creator.");
                return;
            }
            if let Some(p_hash) = self.on_unit_received(su) {
                p_hashes_node_map[NodeIndex(i)] = Some(p_hash);
            } else {
                debug!(target: "env", "In received parent response one of the units is incorrect.");
                return;
            }
        }

        if ControlHash::combine_hashes(&p_hashes_node_map, &self.hashing) != u_chash {
            debug!(target: "env", "In received parent response the control hash is incorrect.");
            return;
        }
        let p_hashes: Vec<H> = p_hashes_node_map.into_iter().flatten().collect();
        self.store.add_parents(u_hash, p_hashes.clone());
        self.send_consensus_notification(NotificationIn::UnitParents(u_hash, p_hashes));
    }

    fn on_network_message(&mut self, message: ConsensusMessage<B, H>, sender: PeerId) {
        match message {
            ConsensusMessage::NewUnit(signed_unit) => {
                self.on_unit_received(signed_unit);
            }
            ConsensusMessage::RequestCoord(coord) => {
                self.on_request_coord(sender, coord);
            }
            ConsensusMessage::ResponseCoord(signed_unit) => {
                debug!(target: "env", "Fetch response received {:?}.", signed_unit);
                self.on_unit_received(signed_unit);
            }
            ConsensusMessage::RequestParents(u_hash) => {
                self.on_request_parents(sender, u_hash);
            }
            ConsensusMessage::ResponseParents(u_hash, parents) => {
                //TODO: these responses are quite heavy, we should at some point add some
                //checks to make sure we are not processing responses to request we did not make.
                //TODO: we need to check if the response does not exceed some max message size in network
                self.on_parents_response(u_hash, parents);
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
        let block_number = match self.client.number(h) {
            Ok(Some(number)) => number,
            _ => {
                error!(target: "env", "a block with hash {} should already be in chain", h);
                return;
            }
        };
        finalize_block(
            self.client.clone(),
            h,
            block_number,
            Some((
                ALEPH_ENGINE_ID,
                AlephJustification::new::<B>(&self.auth_cryptostore, h).encode(),
            )),
        );
    }

    pub(crate) async fn run_epoch(mut self) {
        let mut request_ticker = time::interval(TICK_INTERVAL).fuse();
        loop {
            futures::select! {
                notification = self.rx_consensus.next() => match notification {
                        Some(notification) => self.on_consensus_notification(notification),
                        None => {
                            error!(target: "env", "Consensus notification stream closed.");
                            return;
                        }
                },

                batch = self.rx_order.next() => match batch {
                        Some(batch) => self.on_ordered_batch(batch),
                        None => {
                        error!(target: "env", "Consensus notification stream closed.");
                        return;
                    }
                },

                event = self.rx_network.next() => match event {
                    Some(event) => self.on_network_event(event),
                    None => {
                        error!(target: "env", "Network event stream closed.");
                        return;
                    }
                },
                _ = request_ticker.next() => self.trigger_tasks(),
            }
        }
    }
}

pub(crate) fn finalize_block<BE, B, C>(
    client: Arc<C>,
    hash: B::Hash,
    block_number: NumberFor<B>,
    justification: Option<Justification>,
) where
    B: Block,
    BE: Backend<B>,
    C: crate::ClientForAleph<B, BE>,
{
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
        client.apply_finality(import_op, BlockId::Hash(hash), justification, true)
    });

    let status = client.info();
    debug!(target: "env", "Finalized block with hash {:?}. Current best: #{:?}.", hash, status.finalized_number);
}
