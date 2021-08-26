use crate::{Error, Metrics};
use aleph_bft::OrderedBatch;
use codec::{Decode, Encode};
use futures::channel::{mpsc, oneshot};
use lru::LruCache;
use parking_lot::Mutex;
use sp_consensus::SelectChain;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT, NumberFor};
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    hash::Hash,
    marker::PhantomData,
    sync::Arc,
    time::Duration,
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
const PERIODIC_MAINTENANCE_INTERVAL: Duration = Duration::from_millis(60000);

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

/// This component is used for filtering available data for Aleph Network.
/// It receives new messages for network by `messages_rx` and sends available messages
/// (messages with all blocks already imported by client) by `ready_messages_tx`
pub(crate) struct DataStore<B, C, BE, Message>
where
    B: BlockT,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    BE: Backend<B> + 'static,
    Message: AlephNetworkMessage<B> + std::fmt::Debug,
{
    next_message_id: MessageId,
    ready_messages_tx: UnboundedSender<Message>,
    messages_rx: UnboundedReceiver<Message>,
    dependent_messages: HashMap<AlephDataFor<B>, HashSet<MessageId>>,
    available_blocks: LruCache<AlephDataFor<B>, ()>,
    message_requirements: HashMap<MessageId, usize>,
    pending_messages: HashMap<MessageId, Message>,
    client: Arc<C>,
    _phantom: PhantomData<BE>,
}

impl<B, C, BE, Message> DataStore<B, C, BE, Message>
where
    B: BlockT,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    BE: Backend<B> + 'static,
    Message: AlephNetworkMessage<B> + std::fmt::Debug,
{
    pub(crate) fn new(
        client: Arc<C>,
        ready_messages_tx: UnboundedSender<Message>,
        messages_rx: UnboundedReceiver<Message>,
    ) -> Self {
        DataStore {
            next_message_id: 0,
            client,
            message_requirements: HashMap::new(),
            dependent_messages: HashMap::new(),
            pending_messages: HashMap::new(),
            available_blocks: LruCache::new(AVAILABLE_BLOCKS_CACHE_SIZE),
            ready_messages_tx,
            messages_rx,
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
    /// It keeps `AVAILABLE_BLOCKS_CACHE_SIZE` blocks in cache, remembers messages with
    /// `message_id > highest_message_id - MESSAGE_ID_BOUNDARY` and does periodic check once in
    /// `PERIODIC_MAINTENANCE_INTERVAL`
    pub(crate) async fn run(&mut self, mut exit: oneshot::Receiver<()>) {
        let mut maintenance_timeout = Delay::new(PERIODIC_MAINTENANCE_INTERVAL);
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
                _ = &mut maintenance_timeout => {
                    trace!(target: "afa", "Data Store maintenance timeout");
                    let keys : Vec<_> = self.dependent_messages.keys().cloned().collect();
                    let finalized_number = self.client.info().finalized_number;
                    for block_data in keys {
                        if let Ok(Some(_)) = self.client.header(BlockId::Hash(block_data.hash)) {
                            self.add_block(block_data);
                        } else if finalized_number >= block_data.number {
                            self.add_block(block_data);
                        }
                    }
                    maintenance_timeout = Delay::new(PERIODIC_MAINTENANCE_INTERVAL);
                }
                _ = &mut exit => {
                    break;
                }
            }
        }
    }

    fn forget_message(&mut self, message_id: MessageId) {
        self.message_requirements.remove(&message_id);
        if let Some(message) = self.pending_messages.remove(&message_id) {
            for block_data in message.included_blocks() {
                if let Entry::Occupied(mut entry) = self.dependent_messages.entry(block_data) {
                    entry.get_mut().remove(&message_id);
                    if entry.get().is_empty() {
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
            self.dependent_messages
                .entry(*block_data)
                .or_insert_with(HashSet::new)
                .insert(message_id);
        }
        self.message_requirements
            .insert(message_id, requirements.len());
        self.pending_messages.insert(message_id, message);

        if message_id >= MESSAGE_ID_BOUNDARY {
            self.forget_message(message_id - MESSAGE_ID_BOUNDARY)
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
        if let Some(ids) = self.dependent_messages.remove(&block_data) {
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
        trace!(target: "afa", "Adding block {:?} to Data Store", block_data);
        self.available_blocks.put(block_data, ());
        self.push_messages(block_data);
    }
}

#[derive(Clone)]
pub(crate) struct DataIO<B: BlockT> {
    pub(crate) best_chain: Arc<Mutex<AlephDataFor<B>>>,
    pub(crate) ordered_batch_tx: mpsc::UnboundedSender<OrderedBatch<AlephDataFor<B>>>,
    pub(crate) metrics: Option<Metrics<B::Header>>,
}

pub(crate) async fn refresh_best_chain<B: BlockT, SC: SelectChain<B>>(
    select_chain: SC,
    best_chain: Arc<Mutex<AlephDataFor<B>>>,
    mut exit: oneshot::Receiver<()>,
) {
    loop {
        let delay = futures_timer::Delay::new(Duration::from_millis(REFRESH_INTERVAL));
        tokio::select! {
            _ = delay => {
                let new_best_header = select_chain
                    .best_chain()
                    .await
                    .expect("No best chain");
                *best_chain.lock() = AlephData::new(new_best_header.hash(), *new_best_header.number());
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
        let best = *self.best_chain.lock();

        if let Some(m) = &self.metrics {
            m.report_block(best.hash, std::time::Instant::now(), "get_data");
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
