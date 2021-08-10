use crate::{Error, Metrics};
use aleph_bft::OrderedBatch;
use futures::channel::{mpsc, oneshot};
use lru::LruCache;
use parking_lot::Mutex;
use sp_consensus::SelectChain;
use sp_runtime::traits::{Block as BlockT, Header};
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    marker::PhantomData,
    sync::Arc,
    time::Duration,
};

const REFRESH_INTERVAL: u64 = 100;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_timer::Delay;
use log::{debug, trace};
use sc_client_api::{backend::Backend};
use sp_runtime::generic::BlockId;
use tokio::stream::StreamExt;

type MessageId = u64;
const AVAILABLE_BLOCKS_CACHE_SIZE: usize = 1000;
const MESSAGE_ID_BOUNDARY: MessageId = 100_000;
const PERIODIC_MAINTENANCE_INTERVAL: Duration = Duration::from_millis(60000);

pub(crate) trait AlephNetworkMessage<B: BlockT> {
    fn included_blocks(&self) -> Vec<B::Hash>;
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
    dependent_messages: HashMap<B::Hash, HashSet<MessageId>>,
    available_blocks: LruCache<B::Hash, ()>,
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
                    self.add_block(block.hash);
                }
                _ = &mut maintenance_timeout => {
                    trace!(target: "afa", "Data Store maintenance timeout");
                    let keys : Vec<_> = self.dependent_messages.keys().cloned().collect();
                    for block_hash in keys {
                        if let Ok(Some(_)) = self.client.header(BlockId::Hash(block_hash)) {
                            self.add_block(block_hash);
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
            for block_hash in message.included_blocks() {
                if let Entry::Occupied(mut entry) = self.dependent_messages.entry(block_hash) {
                    entry.get_mut().remove(&message_id);
                    if entry.get().is_empty() {
                        entry.remove_entry();
                    }
                }
            }
        }
    }

    fn add_pending_message(&mut self, message: Message, requirements: Vec<B::Hash>) {
        let message_id = self.next_message_id;
        self.next_message_id += 1;
        for block_hash in requirements.iter() {
            self.dependent_messages
                .entry(*block_hash)
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
        let requirements: Vec<_> = message
            .included_blocks()
            .into_iter()
            .filter(|block_hash| {
                if self.available_blocks.contains(block_hash) {
                    return false;
                }
                if let Ok(Some(_)) = self.client.header(BlockId::Hash(*block_hash)) {
                    self.add_block(*block_hash);
                    return false;
                }
                true
            })
            .collect();

        if requirements.is_empty() {
            trace!(target: "afa", "Sending message from DataStore {:?}", message);
            self.ready_messages_tx
                .unbounded_send(message)
                .expect("Member channel should be open");
        } else {
            self.add_pending_message(message, requirements);
        }
    }

    fn push_messages(&mut self, block_hash: B::Hash) {
        if let Some(ids) = self.dependent_messages.remove(&block_hash) {
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
                    self.ready_messages_tx
                        .unbounded_send(message)
                        .expect("Member channel should be open");
                    self.message_requirements.remove(message_id);
                }
            }
        }
    }

    fn add_block(&mut self, block_hash: B::Hash) {
        trace!(target: "afa", "Adding block {:?} to Data Store", block_hash);
        self.available_blocks.put(block_hash, ());
        self.push_messages(block_hash);
    }
}

#[derive(Clone)]
pub(crate) struct DataIO<B: BlockT> {
    pub(crate) best_chain: Arc<Mutex<B::Hash>>,
    pub(crate) ordered_batch_tx: mpsc::UnboundedSender<OrderedBatch<B::Hash>>,
    pub(crate) metrics: Option<Metrics<B::Header>>,
}

pub(crate) async fn refresh_best_chain<B: BlockT, SC: SelectChain<B>>(
    select_chain: SC,
    best_chain: Arc<Mutex<B::Hash>>,
    mut exit: oneshot::Receiver<()>,
) {
    loop {
        let delay = futures_timer::Delay::new(Duration::from_millis(REFRESH_INTERVAL));
        tokio::select! {
            _ = delay => {
                let new_best_chain = select_chain
                    .best_chain()
                    .await
                    .expect("No best chain")
                    .hash();
                *best_chain.lock() = new_best_chain;
            }
            _ = &mut exit => {
                debug!(target: "afa", "Task for refreshing best chain received exit signal. Terminating.");
                return;
            }
        }
    }
}

impl<B: BlockT> aleph_bft::DataIO<B::Hash> for DataIO<B> {
    type Error = Error;

    fn get_data(&self) -> B::Hash {
        let hash = *self.best_chain.lock();

        if let Some(m) = &self.metrics {
            m.report_block(hash, std::time::Instant::now(), "get_data");
        }
        debug!(target: "afa", "Outputting {:?} in get_data", hash);
        hash
    }

    fn send_ordered_batch(&mut self, batch: OrderedBatch<B::Hash>) -> Result<(), Self::Error> {
        // TODO: add better conversion
        self.ordered_batch_tx
            .unbounded_send(batch)
            .map_err(|_| Error::SendData)
    }
}
