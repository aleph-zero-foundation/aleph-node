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
    messages::{
        sign_unit, Alert, ConsensusMessage, ForkProof, FullUnit, NetworkMessage, SignedUnit,
    },
    network::{NetworkCommand, NetworkEvent},
    AuthorityId, AuthorityKeystore, EpochId,
};
use futures::{channel::mpsc, StreamExt};

use rush::{
    ControlHash, NodeCount, NodeIndex, NodeMap, NotificationIn, NotificationOut, Unit, UnitCoord,
};
use sc_network::PeerId;
use std::collections::{BinaryHeap, HashMap, HashSet};

use crate::justification::AlephJustification;
use futures::stream::Fuse;
use sp_api::NumberFor;
use std::cmp::Ordering;
use tokio::time;

#[cfg(test)]
mod tests;

pub const FETCH_INTERVAL: time::Duration = time::Duration::from_secs(4);
pub const TICK_INTERVAL: time::Duration = time::Duration::from_millis(100);
pub const INITIAL_MULTICAST_DELAY: time::Duration = time::Duration::from_secs(10);
// we will accept units that are of round <= (round_in_progress + ROUNDS_MARGIN) only
pub const ROUNDS_MARGIN: usize = 100;
// TODO: need to make sure we never accept units of round > MAX_ROUND
pub const MAX_ROUND: usize = 5000;
pub const MAX_UNITS_ALERT: usize = 200;

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
    //this is the smallest r, such that round r-1 is saturated, i.e., it has at least threshold (~(2/3)N) units
    round_in_progress: usize,
    threshold: NodeCount,
    //the number of unique nodes that we hold units for a given round
    n_units_per_round: Vec<NodeCount>,
    is_forker: NodeMap<bool>,
    legit_buffer: Vec<SignedUnit<B, H>>,
    hashing: Box<dyn Fn(&[u8]) -> H + Send>,
}

impl<B: Block, H: Hash> UnitStore<B, H> {
    pub(crate) fn new(
        n_nodes: NodeCount,
        threshold: NodeCount,
        hashing: impl Fn(&[u8]) -> H + Send + Copy + 'static,
    ) -> Self {
        UnitStore {
            by_coord: HashMap::new(),
            by_hash: HashMap::new(),
            parents: HashMap::new(),
            round_in_progress: 0,
            threshold,
            n_units_per_round: vec![NodeCount(0); MAX_ROUND + 1],
            // is_forker is initialized with default values for bool, i.e., false
            is_forker: NodeMap::new_with_len(n_nodes),
            legit_buffer: Vec::new(),
            hashing: Box::new(hashing),
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

    // Outputs new legit units that are supposed to be sent to Consensus and emties the buffer.
    pub(crate) fn yield_buffer_units(&mut self) -> Vec<SignedUnit<B, H>> {
        self.legit_buffer.drain(..).collect()
    }

    fn update_round_in_progress(&mut self, candidate_round: usize) {
        if candidate_round >= self.round_in_progress
            && self.n_units_per_round[candidate_round] >= self.threshold
        {
            let old_round = self.round_in_progress;
            self.round_in_progress = candidate_round + 1;
            for round in (old_round + 1)..(self.round_in_progress + 1) {
                for (id, forker) in self.is_forker.enumerate() {
                    if !*forker {
                        let coord = (round, id).into();
                        if let Some(su) = self.unit_by_coord(coord).cloned() {
                            self.legit_buffer.push(su);
                        }
                    }
                }
            }
        }
    }
    // Outputs None if this is not a newly-discovered fork or Some(sv) where (su, sv) form a fork
    pub(crate) fn is_new_fork(&self, su: &SignedUnit<B, H>) -> Option<SignedUnit<B, H>> {
        // TODO: optimize so that unit's hash is computed once only, after it is received
        let hash = su.hash(&self.hashing);
        if self.contains_hash(&hash) {
            return None;
        }
        let coord = su.coord();
        self.unit_by_coord(coord).cloned()
    }

    pub(crate) fn get_round_in_progress(&self) -> usize {
        self.round_in_progress
    }

    pub(crate) fn is_forker(&self, node_id: NodeIndex) -> bool {
        self.is_forker[node_id]
    }

    // Marks a node as a forker and outputs units in store of round <= round_in_progress created by this node.
    // The returned vector is sorted w.r.t. increasing rounds. Units of higher round created by this node are removed from store.
    pub(crate) fn mark_forker(&mut self, forker: NodeIndex) -> Vec<SignedUnit<B, H>> {
        if self.is_forker[forker] {
            error!(target: "env", "Trying to mark the node {:?} as forker for the second time.", forker);
        }
        self.is_forker[forker] = true;
        let forkers_units = (0..=self.round_in_progress)
            .filter_map(|r| self.unit_by_coord((r, forker).into()).cloned())
            .collect();

        for round in self.round_in_progress + 1..=MAX_ROUND {
            let coord = (round, forker).into();
            if let Some(su) = self.unit_by_coord(coord).cloned() {
                // We get rid of this unit. This is safe because it has not been sent to Consensus yet.
                // The reason we do that, is to be in a "clean" situation where we alert all forker's
                // units in the store and the only way this forker's unit is sent to Consensus is when
                // it arrives in an alert for the *first* time.
                // If we didn't do that, then there would be some awkward issues with duplicates.
                self.by_coord.remove(&coord);
                let hash = su.hash(&self.hashing);
                self.by_hash.remove(&hash);
                self.parents.remove(&hash);
                // Now we are in a state as if the unit never arrived.
            }
        }
        forkers_units
    }

    pub(crate) fn add_unit(&mut self, su: SignedUnit<B, H>, alert: bool) {
        // TODO: optimize so that unit's hash is computed once only, after it is received
        let hash = su.hash(&self.hashing);
        let round = su.round();
        let creator = su.creator();
        if alert {
            assert!(
                self.is_forker[creator],
                "The forker must be marked before adding alerted units."
            );
        }
        if self.contains_hash(&hash) {
            // Ignoring a duplicate.
            return;
        }
        self.by_hash.insert(hash, su.clone());
        let coord = su.coord();
        // We do not store multiple forks of a unit by coord, as there is never a need to
        // fetch all units corresponding to a particular coord.
        if self.by_coord.insert(coord, su.clone()).is_none() {
            // This means that this unit is not a fork (even though the creator might be a forker)
            self.n_units_per_round[round] += NodeCount(1);
        }
        // NOTE: a minor inefficiency is that we send alerted units of high rounds that are possibly
        // way beyond round_in_progress right away to Consensus. This could be perhaps corrected so that
        // we wait until the round is in progress, but this does not seem to help vs actual attacks and in
        // "accidental" forks the rounds will never be much higher than round_in_progress.
        if alert || (round <= self.round_in_progress && !self.is_forker[creator]) {
            self.legit_buffer.push(su);
        }
        self.update_round_in_progress(round);
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
    n_nodes: NodeCount,
    threshold: NodeCount,
    our_node_ix: NodeIndex,
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
        our_node_ix: NodeIndex,
    ) -> Self {
        let n_nodes = NodeCount(authorities.len());
        let threshold = (n_nodes * 2) / 3 + NodeCount(1);
        Environment {
            client,
            select_chain,
            tx_consensus,
            rx_consensus,
            tx_network,
            rx_network,
            rx_order: rx_order.fuse(),
            auth_cryptostore,
            store: UnitStore::new(n_nodes, threshold, hashing),
            requests: BinaryHeap::new(),
            hashing: Box::new(hashing),
            epoch_id,
            n_nodes,
            threshold,
            our_node_ix,
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
        self.store.add_unit(signed_unit, false);
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

    // Pulls tasks from the priority queue (sorted by scheduled time) and sends them to random peers
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
        // NOTE: we double the delay each time
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

    fn validate_unit_parents(&self, su: &SignedUnit<B, H>) -> bool {
        // TODO: it might be cleaner to move this to rush, this only needs to know the total number of nodes
        // NOTE: at this point we cannot validate correctness of the control hash, in principle it could be
        // just a random hash, but we still would not be able to deduce that by looking at the unit only.
        let control_hash = &su.unit.inner.control_hash;
        let round = su.round();
        let creator = su.creator();
        if su.n_members() != self.n_nodes {
            debug!(target: "env", "Unit with wrong length of parents map.");
            return false;
        }
        let n_parents = su.n_parents();
        let threshold = self.threshold;
        if round == 0 && n_parents > NodeCount(0) {
            debug!(target: "env", "Unit of round zero with non-zero number of parents.");
            return false;
        }
        if round > 0 && n_parents < threshold {
            debug!(target: "env", "Unit of non-zero round with only {:?} parents while at least {:?} are required.", n_parents, threshold);
            return false;
        }
        if round > 0 && !control_hash.parents[creator] {
            debug!(target: "env", "Unit does not have its creator's previous unit as parent.");
            return false;
        }
        true
    }

    fn validate_unit(&self, su: &SignedUnit<B, H>) -> bool {
        // TODO: make sure we check all that is necessary for unit correctness
        // TODO: consider moving validation logic for units and alerts to another file, note however
        // that access to the authority list is required for validation.
        if su.unit.epoch_id != self.epoch_id {
            // NOTE: this implies malicious behavior as the unit's epoch_id
            // is incompatible with epoch_id of the message it arrived in.
            debug!(target: "env", "A unit with incorrect epoch_id! {:?}", su);
            return false;
        }
        if !su.verify_unit_signature() {
            debug!(target: "env", "A unit with incorrect signature! {:?}", su);
            return false;
        }
        if su.round() > MAX_ROUND {
            debug!(target: "env", "A unit with too high round {}! {:?}", su.round(), su);
            return false;
        }
        if su.creator().0 >= self.n_nodes.0 {
            debug!(target: "env", "A unit with too high creator index {}! {:?}", su.creator(), su);
            return false;
        }
        if !self.validate_unit_parents(su) {
            debug!(target: "env", "A unit did not pass parents validation. {:?}", su);
            return false;
        }
        true
    }

    fn add_unit_to_store_unless_fork(&mut self, su: SignedUnit<B, H>) {
        if let Some(sv) = self.store.is_new_fork(&su) {
            let creator = su.creator();
            if !self.store.is_forker(creator) {
                // We need to mark the forker if it is not known yet.
                let proof = ForkProof { u1: su, u2: sv };
                self.on_new_forker_detected(creator, proof);
            }
            // We ignore this unit. If it is legit, it will arrive in some alert and we need to wait anyway.
            // There is no point in keeping this unit in any kind of buffer.
            return;
        }
        let u_round = su.round();
        let round_in_progress = self.store.get_round_in_progress();
        if u_round <= round_in_progress + ROUNDS_MARGIN {
            self.store.add_unit(su, false);
        } else {
            debug!(target: "env", "Unit {:?} ignored because of too high round {} when round in progress is {}.", su, u_round, round_in_progress);
        }
    }

    fn move_units_to_consensus(&mut self) {
        let mut units = Vec::new();
        for su in self.store.yield_buffer_units() {
            let hash = su.hash(&self.hashing);
            let unit = Unit::new_from_preunit(su.unit.inner.clone(), hash);
            units.push(unit);
        }
        if !units.is_empty() {
            self.send_consensus_notification(NotificationIn::NewUnits(units));
        }
    }

    fn on_unit_received(&mut self, su: SignedUnit<B, H>, alert: bool) {
        if alert {
            // The unit has been validated already, we add to store.
            self.store.add_unit(su, true);
        } else if self.validate_unit(&su) {
            self.add_unit_to_store_unless_fork(su);
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
        // TODO: we *must* make sure that we have indeed sent such a request before accepting the response.
        let maybe_u = self.store.unit_by_hash(&u_hash);
        if maybe_u.is_none() {
            debug!(target: "env", "We got parents but don't even know the unit. Ignoring.");
            return;
        }
        let u = maybe_u.unwrap();
        let u_round = u.round();
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

        let mut p_hashes_node_map: NodeMap<Option<H>> = NodeMap::new_with_len(self.n_nodes);
        for (i, su) in parents.into_iter().enumerate() {
            if su.round() + 1 != u_round {
                debug!(target: "env", "In received parent response received a unit with wrong round.");
                return;
            }
            if su.creator() != parent_ids[i] {
                debug!(target: "env", "In received parent response received a unit with wrong creator.");
                return;
            }
            if !self.validate_unit(&su) {
                debug!(target: "env", "In received parent response received a unit that does not pass validation.");
                return;
            }
            let p_hash = su.hash(&self.hashing);
            p_hashes_node_map[NodeIndex(i)] = Some(p_hash);
            // There might be some optimization possible here to not validate twice, but overall
            // this piece of code should be executed extremely rarely.
            self.add_unit_to_store_unless_fork(su);
        }

        if ControlHash::combine_hashes(&p_hashes_node_map, &self.hashing) != u_chash {
            debug!(target: "env", "In received parent response the control hash is incorrect.");
            return;
        }
        let p_hashes: Vec<H> = p_hashes_node_map.into_iter().flatten().collect();
        self.store.add_parents(u_hash, p_hashes.clone());
        self.send_consensus_notification(NotificationIn::UnitParents(u_hash, p_hashes));
    }

    fn validate_fork_proof(&self, forker: NodeIndex, proof: &ForkProof<B, H>) -> bool {
        if !self.validate_unit(&proof.u1) || !self.validate_unit(&proof.u2) {
            debug!(target: "env", "One of the units in the proof is invalid.");
            return false;
        }
        if proof.u1.creator() != forker || proof.u2.creator() != forker {
            debug!(target: "env", "One of the units creators in proof does not match.");
            return false;
        }
        if proof.u1.round() != proof.u2.round() {
            debug!(target: "env", "The rounds in proof's units do not match.");
            return false;
        }
        true
    }

    fn validate_alerted_units(&self, forker: NodeIndex, units: &[SignedUnit<B, H>]) -> bool {
        // Correctness rules:
        // 1) All units must pass unit validation
        // 2) All units must be created by forker
        // 3) All units must come from different rounds
        // 4) There must be <= MAX_UNITS_ALERT of them
        if units.len() > MAX_UNITS_ALERT {
            debug!(target: "env", "Too many units: {} included in alert.", units.len());
            return false;
        }
        let mut rounds: HashSet<usize> = HashSet::new();
        for u in units {
            if u.creator() != forker {
                debug!(target: "env", "One of the units {:?} has wrong creator.", u);
                return false;
            }
            if !self.validate_unit(u) {
                debug!(target: "env", "One of the units {:?} in alert does not pass validation.", u);
                return false;
            }
            if rounds.contains(&u.round()) {
                debug!(target: "env", "Two or more alerted units have the same round {:?}.", u.round());
                return false;
            }
            rounds.insert(u.round());
        }
        true
    }

    fn validate_alert(&self, alert: &Alert<B, H>) -> bool {
        // The correctness of forker and sender should be checked in RBC, but no harm
        // to have a check here as well for now.
        if alert.forker.0 >= self.n_nodes.0 {
            debug!(target: "env", "Alert has incorrect forker field {:?}", alert.forker);
            return false;
        }
        if alert.sender.0 >= self.n_nodes.0 {
            debug!(target: "env", "Alert has incorrect sender field {:?}", alert.sender);
            return false;
        }
        if !self.validate_fork_proof(alert.forker, &alert.proof) {
            debug!(target: "env", "Alert has incorrect fork proof.");
            return false;
        }
        if !self.validate_alerted_units(alert.forker, &alert.legit_units) {
            debug!(target: "env", "Alert has incorrect unit/s.");
            return false;
        }
        true
    }

    fn form_alert(
        &self,
        forker: NodeIndex,
        proof: ForkProof<B, H>,
        units: Vec<SignedUnit<B, H>>,
    ) -> Alert<B, H> {
        Alert {
            sender: self.our_node_ix,
            forker,
            proof,
            legit_units: units,
        }
    }

    fn on_new_forker_detected(&mut self, forker: NodeIndex, proof: ForkProof<B, H>) {
        let mut alerted_units = self.store.mark_forker(forker);
        if alerted_units.len() > MAX_UNITS_ALERT {
            // The ordering is increasing w.r.t. rounds.
            alerted_units.reverse();
            alerted_units.truncate(MAX_UNITS_ALERT);
            alerted_units.reverse();
        }
        let alert = self.form_alert(forker, proof, alerted_units);
        let message = self.form_network_message(ConsensusMessage::ForkAlert(alert));
        let command = NetworkCommand::ReliableBroadcast(message);
        self.place_network_command(command);
    }

    fn on_fork_alert(&mut self, alert: Alert<B, H>) {
        if self.validate_alert(&alert) {
            let forker = alert.forker;
            if !self.store.is_forker(forker) {
                // We learn about this forker for the first time, need to send our own alert
                self.on_new_forker_detected(forker, alert.proof);
            }
            for su in alert.legit_units {
                self.on_unit_received(su, true);
            }
        } else {
            debug!(
                "We have received an incorrect alert from {} on forker {}.",
                alert.sender, alert.forker
            );
        }
    }

    fn on_network_message(&mut self, message: ConsensusMessage<B, H>, sender: PeerId) {
        match message {
            ConsensusMessage::NewUnit(signed_unit) => {
                self.on_unit_received(signed_unit, false);
            }
            ConsensusMessage::RequestCoord(coord) => {
                self.on_request_coord(sender, coord);
            }
            ConsensusMessage::ResponseCoord(signed_unit) => {
                debug!(target: "env", "Fetch response received {:?}.", signed_unit);
                self.on_unit_received(signed_unit, false);
            }
            ConsensusMessage::RequestParents(u_hash) => {
                self.on_request_parents(sender, u_hash);
            }
            ConsensusMessage::ResponseParents(u_hash, parents) => {
                // TODO: these responses are quite heavy, we should at some point add
                // checks to make sure we are not processing responses to request we did not make.
                // TODO: we need to check if the response (and alert) does not exceed some max message size in network.
                self.on_parents_response(u_hash, parents);
            }
            ConsensusMessage::ForkAlert(alert) => {
                self.on_fork_alert(alert);
            }
        }
    }

    fn on_network_event(&mut self, event: NetworkEvent<B, H>) {
        match event {
            NetworkEvent::MessageReceived(message, sender) => {
                self.on_network_message(message, sender);
            }
            NetworkEvent::PeerConnected(peer_id) => {
                // TODO: might want to add support for this
                debug!("New peer connected: {:?}.", peer_id);
            }
            NetworkEvent::PeerDisconnected(peer_id) => {
                // TODO: might want to add support for this
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
        let mut ticker = time::interval(TICK_INTERVAL).fuse();
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
                _ = ticker.next() => self.trigger_tasks(),
            }
            self.move_units_to_consensus();
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
