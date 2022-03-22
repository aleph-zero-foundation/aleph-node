use crate::{
    aggregation::multicast::{Hash, Multicast, Multisigned, SignableHash},
    metrics::Checkpoint,
    network::DataNetwork,
    Metrics,
};
use aleph_bft::{MultiKeychain, Recipient, Signable};
use codec::Codec;
use futures::{channel::mpsc, StreamExt};
use log::{debug, trace, warn};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    marker::PhantomData,
};

pub enum AggregatorError {
    LastHashPlaced,
    NoHashFound,
    DuplicateHash,
}

pub enum IOError {
    NetworkChannelClosed,
}

pub type AggregatorResult<R> = Result<R, AggregatorError>;
pub type IOResult = Result<(), IOError>;

/// A wrapper around an `rmc::Multicast` returning the signed hashes in the order of the [`Multicast::start_multicast`] calls.
pub struct BlockSignatureAggregator<
    'a,
    H: Hash + Copy,
    MK: MultiKeychain,
    MS: Multisigned<'a, SignableHash<H>, MK>,
> {
    signatures: HashMap<H, MK::PartialMultisignature>,
    hash_queue: VecDeque<H>,
    last_hash_placed: bool,
    started_hashes: HashSet<H>,
    metrics: Option<Metrics<H>>,
    marker: PhantomData<&'a MS>,
}

impl<'a, H: Copy + Hash, MK: MultiKeychain, MS: Multisigned<'a, SignableHash<H>, MK>>
    BlockSignatureAggregator<'a, H, MK, MS>
{
    pub(crate) fn new(metrics: Option<Metrics<H>>) -> Self {
        BlockSignatureAggregator {
            signatures: HashMap::new(),
            hash_queue: VecDeque::new(),
            last_hash_placed: false,
            started_hashes: HashSet::new(),
            metrics,
            marker: PhantomData,
        }
    }

    fn on_start(&mut self, hash: H) -> AggregatorResult<()> {
        if !self.started_hashes.insert(hash) {
            return Err(AggregatorError::DuplicateHash);
        }
        if let Some(metrics) = &self.metrics {
            metrics.report_block(hash, std::time::Instant::now(), Checkpoint::Aggregating);
        }
        self.hash_queue.push_back(hash);

        Ok(())
    }

    pub(crate) fn notify_last_hash(&mut self) {
        self.last_hash_placed = true;
    }

    fn on_multisigned_hash(&mut self, multisigned_hash: MS) {
        let hash = multisigned_hash.as_signable().hash();
        let unchecked = multisigned_hash.into_unchecked();
        debug!(target: "aleph-aggregator", "New multisigned_hash {:?}.", unchecked);
        self.signatures.insert(hash, unchecked.signature());
    }

    fn try_pop_hash(&mut self) -> AggregatorResult<(H, MK::PartialMultisignature)> {
        match self.hash_queue.pop_front() {
            Some(hash) => {
                if let Some(multisignature) = self.signatures.remove(&hash) {
                    Ok((hash, multisignature))
                } else {
                    self.hash_queue.push_front(hash);
                    Err(AggregatorError::NoHashFound)
                }
            }
            None => {
                if self.last_hash_placed {
                    Err(AggregatorError::LastHashPlaced)
                } else {
                    Err(AggregatorError::NoHashFound)
                }
            }
        }
    }
}

pub struct IO<
    'a,
    H: Hash + Copy,
    D: Clone + Codec + Debug + Send + Sync + 'static,
    N: DataNetwork<D>,
    MK: MultiKeychain,
    RMC: Multicast<H, SignableHash<H>>,
> where
    RMC::Signed: Multisigned<'a, SignableHash<H>, MK>,
{
    messages_for_rmc: mpsc::UnboundedSender<D>,
    messages_from_rmc: mpsc::UnboundedReceiver<D>,
    network: N,
    multicast: RMC,
    aggregator: BlockSignatureAggregator<'a, H, MK, RMC::Signed>,
}

impl<
        'a,
        H: Copy + Hash,
        D: Clone + Codec + Debug + Send + Sync,
        N: DataNetwork<D>,
        MK: MultiKeychain,
        RMC: Multicast<H, SignableHash<H>>,
    > IO<'a, H, D, N, MK, RMC>
where
    RMC::Signed: Multisigned<'a, SignableHash<H>, MK>,
{
    pub(crate) fn new(
        messages_for_rmc: mpsc::UnboundedSender<D>,
        messages_from_rmc: mpsc::UnboundedReceiver<D>,
        network: N,
        multicast: RMC,
        aggregator: BlockSignatureAggregator<'a, H, MK, RMC::Signed>,
    ) -> Self {
        IO {
            messages_for_rmc,
            messages_from_rmc,
            network,
            multicast,
            aggregator,
        }
    }

    pub(crate) async fn start_aggregation(&mut self, hash: H) {
        debug!(target: "aleph-aggregator", "Started aggregation for block hash {:?}", hash);
        if let Err(AggregatorError::DuplicateHash) = self.aggregator.on_start(hash) {
            debug!(target: "aleph-aggregator", "Aggregation already started for block hash {:?}, ignoring.", hash);
            return;
        }
        self.multicast
            .start_multicast(SignableHash::new(hash))
            .await;
    }

    pub fn notify_last_hash(&mut self) {
        self.aggregator.notify_last_hash()
    }

    async fn wait_for_next_signature(&mut self) -> IOResult {
        loop {
            tokio::select! {
                multisigned_hash = self.multicast.next_multisigned_hash() => {
                    self.aggregator.on_multisigned_hash(multisigned_hash);
                    return Ok(());
                }
                message_from_rmc = self.messages_from_rmc.next() => {
                    trace!(target: "aleph-aggregator", "Our rmc message {:?}.", message_from_rmc);
                    match message_from_rmc {
                        Some(message_from_rmc) => {
                            self.network.send(message_from_rmc, Recipient::Everyone)
                                        .expect("sending message from rmc failed");
                        },
                        None => {
                            warn!(target: "aleph-aggregator", "the channel of messages from rmc closed");
                        }
                    }
                }
                message_from_network = self.network.next() =>
                    match message_from_network {
                        Some(message_from_network) => {
                            trace!(target: "aleph-aggregator", "Received message for rmc: {:?}", message_from_network);
                            self.messages_for_rmc.unbounded_send(message_from_network)
                                                 .expect("sending message to rmc failed");
                        },
                        None => {
                            // In case the network is down we can terminate (?).
                            return Err(IOError::NetworkChannelClosed);
                        }
                    }
            }
        }
    }

    pub(crate) async fn next_multisigned_hash(&mut self) -> Option<(H, MK::PartialMultisignature)> {
        loop {
            trace!(target: "aleph-aggregator", "Entering next_multisigned_hash loop.");
            match self.aggregator.try_pop_hash() {
                Ok(res) => {
                    return Some(res);
                }
                Err(AggregatorError::LastHashPlaced) => {
                    debug!(
                        target: "aleph-aggregator",
                        "Terminating next_multisigned_hash because the last hash has been signed."
                    );
                    return None;
                }
                Err(AggregatorError::NoHashFound) => { /* ignored */ }
                Err(AggregatorError::DuplicateHash) => {
                    warn!(
                        target: "aleph-aggregator",
                        "Unexpected aggregator exception in IO: DuplicateHash",
                    )
                }
            }

            if self.wait_for_next_signature().await.is_err() {
                warn!(target: "aleph-aggregator", "the network channel closed");
                return None;
            }
        }
    }
}
