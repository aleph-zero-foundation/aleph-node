use crate::{metrics::Checkpoint, network, Error, Metrics};
use aleph_bft::OrderedBatch;
use codec::{Decode, Encode};
use futures::channel::{mpsc, oneshot};
use lru::LruCache;
use parking_lot::Mutex;
use sp_consensus::SelectChain;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT, NumberFor};
use std::default::Default;
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    hash::Hash,
    marker::PhantomData,
    sync::Arc,
    time::{self, Duration},
};

const REFRESH_INTERVAL: u64 = 100;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_timer::Delay;
use log::{debug, trace};
use sc_client_api::backend::Backend;
use sp_runtime::generic::BlockId;
use tokio::stream::StreamExt;

type MessageId = u64;
const AVAILABLE_BLOCKS_CACHE_SIZE: usize = 1000;
const MESSAGE_ID_BOUNDARY: MessageId = 100_000;
const PERIODIC_MAINTENANCE_INTERVAL: Duration = Duration::from_secs(60);
const REQUEST_FORK_AFTER: Duration = Duration::from_secs(100);

#[derive(Copy, PartialEq, Eq, Clone, Debug, Encode, Decode, Hash)]
pub(crate) struct AlephData<H, N> {
    pub hash: H,
    pub number: N,
}

impl<H, N> AlephData<H, N> {
    pub(crate) fn new(block_hash: H, block_number: N) -> Self {
        AlephData {
            hash: block_hash,
            number: block_number,
        }
    }
}

pub(crate) type AlephDataFor<B> = AlephData<<B as BlockT>::Hash, NumberFor<B>>;

pub(crate) trait AlephNetworkMessage<B: BlockT> {
    fn included_blocks(&self) -> Vec<AlephDataFor<B>>;
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub(crate) struct MissingBlockInfo {
    // Which messages are being held because of a missing block.
    messages: HashSet<MessageId>,
    // When was the first message containing this block encountered.
    first_occurence: time::SystemTime,
}

impl MissingBlockInfo {
    fn new() -> Self {
        MissingBlockInfo {
            messages: HashSet::new(),
            first_occurence: time::SystemTime::now(),
        }
    }
}

pub struct DataStoreConfig {
    pub available_blocks_cache_capacity: usize,
    pub message_id_boundary: MessageId,
    pub periodic_maintenance_interval: Duration,
    pub request_block_after: Duration,
}

impl Default for DataStoreConfig {
    fn default() -> DataStoreConfig {
        DataStoreConfig {
            available_blocks_cache_capacity: AVAILABLE_BLOCKS_CACHE_SIZE,
            message_id_boundary: MESSAGE_ID_BOUNDARY,
            periodic_maintenance_interval: PERIODIC_MAINTENANCE_INTERVAL,
            request_block_after: REQUEST_FORK_AFTER,
        }
    }
}

/// This component is used for filtering available data for Aleph Network.
/// It receives new messages for network by `messages_rx` and sends available messages
/// (messages with all blocks already imported by client) by `ready_messages_tx`
pub(crate) struct DataStore<B, C, BE, RB, Message>
where
    B: BlockT,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    BE: Backend<B> + 'static,
    RB: network::RequestBlocks<B> + 'static,
    Message: AlephNetworkMessage<B> + std::fmt::Debug,
{
    next_message_id: MessageId,
    ready_messages_tx: UnboundedSender<Message>,
    messages_rx: UnboundedReceiver<Message>,
    missing_blocks: HashMap<AlephDataFor<B>, MissingBlockInfo>,
    available_blocks: LruCache<AlephDataFor<B>, ()>,
    message_requirements: HashMap<MessageId, usize>,
    pending_messages: HashMap<MessageId, Message>,
    client: Arc<C>,
    block_requester: RB,
    config: DataStoreConfig,
    _phantom: PhantomData<BE>,
}

impl<B, C, BE, RB, Message> DataStore<B, C, BE, RB, Message>
where
    B: BlockT,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    BE: Backend<B> + 'static,
    RB: network::RequestBlocks<B> + 'static,
    Message: AlephNetworkMessage<B> + std::fmt::Debug,
{
    pub(crate) fn new(
        client: Arc<C>,
        block_requester: RB,
        ready_messages_tx: UnboundedSender<Message>,
        messages_rx: UnboundedReceiver<Message>,
        config: DataStoreConfig,
    ) -> Self {
        DataStore {
            next_message_id: 0,
            client,
            block_requester,
            message_requirements: HashMap::new(),
            missing_blocks: HashMap::new(),
            pending_messages: HashMap::new(),
            available_blocks: LruCache::new(config.available_blocks_cache_capacity),
            ready_messages_tx,
            messages_rx,
            config,
            _phantom: PhantomData,
        }
    }

    /// This method is used for running DataStore. It polls on 4 things:
    /// 1. Receives AlephNetworkMessage and either sends it further if message is available or saves it for later
    /// 2. Receives newly imported blocks and sends all messages that are available because of this block further
    /// 3. Periodically checks for saved massages that are available and sends them further
    /// 4. Waits for exit signal
    /// This component on each new imported block stores it in cache. There is no guarantee, that all blocks will
    /// be received from notification stream, so there is a periodic check for all needed blocks.
    /// It keeps `config.available_blocks_cache_capacity` blocks in cache, remembers messages with
    /// `message_id > highest_message_id - config.message_id_boundary` and does periodic check once in
    /// `config.periodic_maintenance_interval`
    /// In case a block is missing for more than `config.request_block_after` then we request it via a
    /// `request_stale_block` call -- this happens in the periodic maintenance.
    pub(crate) async fn run(&mut self, mut exit: oneshot::Receiver<()>) {
        let mut maintenance_clock = Delay::new(self.config.periodic_maintenance_interval);
        let mut import_stream = self.client.import_notification_stream();
        loop {
            tokio::select! {
                Some(message) = &mut self.messages_rx.next() => {
                    trace!(target: "afa", "Received message at Data Store {:?}", message);
                    self.add_message(message);
                }
                Some(block) = &mut import_stream.next() => {
                    trace!(target: "afa", "Block import notification at Data Store for block {:?}", block);
                    // Here we don't handle messages with incorrect number (number different
                    // than `*block.header.number()`). This will be delt with by maintenance
                    // as data containing `(hash, incorrect_number)` will be sent
                    // forward if client has imported `hash`
                    self.add_block(AlephData::new(block.hash, *block.header.number()));
                }
                _ = &mut maintenance_clock => {
                    self.run_maintenance();

                    maintenance_clock = Delay::new(self.config.periodic_maintenance_interval);
                }
                _ = &mut exit => {
                    break;
                }
            }
        }
    }

    fn run_maintenance(&mut self) {
        let blocks_with_timestamps: Vec<_> = self
            .missing_blocks
            .iter()
            .map(|(block, info)| (*block, info.first_occurence))
            .collect();
        debug!(target: "afa", "Data Store maintanance. Awaiting {:?} blocks: {:?}", blocks_with_timestamps.len(), blocks_with_timestamps);
        let finalized_number = self.client.info().finalized_number;
        let now = time::SystemTime::now();
        for (block_data, first_occurence) in blocks_with_timestamps {
            if let Ok(Some(_)) = self.client.header(BlockId::Hash(block_data.hash)) {
                self.add_block(block_data);
            } else if finalized_number >= block_data.number {
                self.add_block(block_data);
            } else if let Ok(time_waiting) = now.duration_since(first_occurence) {
                if time_waiting >= self.config.request_block_after {
                    debug!(target: "afa", "Requesting a stale block {:?} after it has been missing for {:?} secs.", block_data, time_waiting.as_secs());
                    // Requesting a block multiple times is fine -- duplicate requests are not registered internally,
                    // the old request is only updated. However, the substrate documentation suggests to not request
                    // recent blocks this way (only stale forks), hence we wait at least 100 secs before attempting one.
                    self.block_requester
                        .request_stale_block(block_data.hash, block_data.number);
                }
            }
        }
    }

    fn forget_message(&mut self, message_id: MessageId) {
        self.message_requirements.remove(&message_id);
        if let Some(message) = self.pending_messages.remove(&message_id) {
            for block_data in message.included_blocks() {
                if let Entry::Occupied(mut entry) = self.missing_blocks.entry(block_data) {
                    entry.get_mut().messages.remove(&message_id);
                    if entry.get().messages.is_empty() {
                        entry.remove_entry();
                    }
                }
            }
        }
    }

    fn add_pending_message(&mut self, message: Message, requirements: Vec<AlephDataFor<B>>) {
        let message_id = self.next_message_id;
        self.next_message_id += 1;
        for block_data in requirements.iter() {
            self.missing_blocks
                .entry(*block_data)
                .or_insert_with(MissingBlockInfo::new)
                .messages
                .insert(message_id);
        }
        self.message_requirements
            .insert(message_id, requirements.len());
        self.pending_messages.insert(message_id, message);

        // Below we remove the message with id `self.next_message_id - self.config.message_id_boundary` to keep the invariant
        // that the set of messages kept are the ones with ids in the interval
        // [self.next_message_id - self.config.message_id_boundary, self.next_message_id-1].
        // Note that `add_pending_message` is the only method that modifies `self.next_message_id`, this works as expected.
        if message_id >= self.config.message_id_boundary {
            self.forget_message(message_id - self.config.message_id_boundary)
        }
    }

    fn add_message(&mut self, message: Message) {
        let finalized_number = self.client.info().finalized_number;
        let requirements: Vec<_> = message
            .included_blocks()
            .into_iter()
            .filter(|block_data| {
                if self.available_blocks.contains(block_data) {
                    return false;
                }
                if let Ok(Some(_)) = self.client.header(BlockId::Hash(block_data.hash)) {
                    self.add_block(*block_data);
                    return false;
                }
                if finalized_number >= block_data.number {
                    self.add_block(*block_data);
                    return false;
                }
                true
            })
            .collect();

        if requirements.is_empty() {
            trace!(target: "afa", "Sending message from DataStore {:?}", message);
            if let Err(e) = self.ready_messages_tx.unbounded_send(message) {
                debug!(target: "afa", "Unable to send a ready message from DataStore {}", e);
            }
        } else {
            self.add_pending_message(message, requirements);
        }
    }

    fn push_messages(&mut self, block_data: AlephDataFor<B>) {
        if let Some(MissingBlockInfo {
            messages: ids,
            first_occurence: _,
        }) = self.missing_blocks.remove(&block_data)
        {
            for message_id in ids.iter() {
                *self
                    .message_requirements
                    .get_mut(message_id)
                    .expect("there are some requirements") -= 1;
                if self.message_requirements[message_id] == 0 {
                    let message = self
                        .pending_messages
                        .remove(message_id)
                        .expect("there is a pending message");
                    if let Err(e) = self.ready_messages_tx.unbounded_send(message) {
                        debug!(target: "afa", "Unable to send a ready message from DataStore {}", e);
                    }
                    self.message_requirements.remove(message_id);
                }
            }
        }
    }

    fn add_block(&mut self, block_data: AlephDataFor<B>) {
        debug!(target: "afa", "Adding block {:?} to Data Store", block_data);
        self.available_blocks.put(block_data, ());
        self.push_messages(block_data);
    }
}

#[derive(Clone)]
pub(crate) struct DataIO<B: BlockT> {
    pub(crate) proposed_block: Arc<Mutex<AlephDataFor<B>>>,
    pub(crate) ordered_batch_tx: mpsc::UnboundedSender<OrderedBatch<AlephDataFor<B>>>,
    pub(crate) metrics: Option<Metrics<B::Header>>,
}

// Reduce block header to the level given by num, by traversing down via parents.
pub(crate) fn reduce_header_to_num<B, BE, C>(
    client: Arc<C>,
    header: B::Header,
    num: NumberFor<B>,
) -> B::Header
where
    B: BlockT,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    BE: Backend<B> + 'static,
{
    let mut curr_header = header;
    while curr_header.number() > &num {
        curr_header = client
            .header(BlockId::Hash(*curr_header.parent_hash()))
            .expect("client must respond")
            .expect("parent hash is known by the client");
    }
    curr_header
}

pub(crate) async fn refresh_best_chain<B, BE, SC, C>(
    select_chain: SC,
    client: Arc<C>,
    proposed_block: Arc<Mutex<AlephDataFor<B>>>,
    max_block_num: NumberFor<B>,
    mut exit: oneshot::Receiver<()>,
) where
    B: BlockT,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
{
    // We would like proposed_block to contain the highest ancestor of the `best_block` (this is what
    // `select_chain` provides us with) up to the maximal height of `max_block_num`. This task periodically
    // queries `select_chain` for the `best_block` and updates proposed_block accordingly. One optimization that it
    // uses is that once the block in `proposed_block` reaches the height of `max_block_num`, and the just queried
    // `best_block` is a `descendant` of the previous query, then we don't need to update proposed_block, as it is
    // already correct.
    let mut prev_best_hash: B::Hash = proposed_block.lock().hash;
    let mut prev_best_number: NumberFor<B> = proposed_block.lock().number;
    loop {
        let delay = futures_timer::Delay::new(Duration::from_millis(REFRESH_INTERVAL));
        tokio::select! {
            _ = delay => {
                let new_best_header = select_chain
                    .best_chain()
                    .await
                    .expect("No best chain");
                if *new_best_header.number() <= max_block_num {
                    *proposed_block.lock() = AlephData::new(new_best_header.hash(), *new_best_header.number());
                } else {
                    // we check if prev_best_header is an ancestor of new_best_header:
                    if proposed_block.lock().number < max_block_num {
                        let reduced_header = reduce_header_to_num(client.clone(), new_best_header.clone(), max_block_num);
                        *proposed_block.lock() = AlephData::new(reduced_header.hash(), *reduced_header.number());
                    } else {
                        let reduced_header = reduce_header_to_num(client.clone(), new_best_header.clone(), prev_best_number);
                        if reduced_header.hash() != prev_best_hash {
                            // the new best block is not a descendant of previous best
                            let reduced_header = reduce_header_to_num(client.clone(), new_best_header.clone(), max_block_num);
                            *proposed_block.lock() = AlephData::new(reduced_header.hash(), *reduced_header.number());
                        }
                    }
                }
                prev_best_hash = new_best_header.hash();
                prev_best_number = *new_best_header.number();
            }
            _ = &mut exit => {
                debug!(target: "afa", "Task for refreshing best chain received exit signal. Terminating.");
                return;
            }
        }
    }
}

impl<B: BlockT> aleph_bft::DataIO<AlephDataFor<B>> for DataIO<B> {
    type Error = Error;

    fn get_data(&self) -> AlephDataFor<B> {
        let best = *self.proposed_block.lock();

        if let Some(m) = &self.metrics {
            m.report_block(best.hash, std::time::Instant::now(), Checkpoint::Ordering);
        }
        debug!(target: "afa", "Outputting {:?} in get_data", best);
        best
    }

    fn send_ordered_batch(
        &mut self,
        batch: OrderedBatch<AlephDataFor<B>>,
    ) -> Result<(), Self::Error> {
        // TODO: add better conversion
        self.ordered_batch_tx
            .unbounded_send(batch)
            .map_err(|_| Error::SendData)
    }
}
