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

pub(crate) enum AggregatorError {
    NetworkChannelClosed,
}

/// A wrapper around an `rmc::Multicast` returning the signed hashes in the order of the [`Multicast::start_multicast`] calls.
pub struct BlockSignatureAggregator<
    'a,
    H: Hash + Copy,
    D: Clone + Codec + Debug + Send + Sync + 'static,
    N: DataNetwork<D>,
    MK: MultiKeychain,
    RMC: Multicast<H, SignableHash<H>>,
> {
    messages_for_rmc: mpsc::UnboundedSender<D>,
    messages_from_rmc: mpsc::UnboundedReceiver<D>,
    signatures: HashMap<H, MK::PartialMultisignature>,
    hash_queue: VecDeque<H>,
    network: N,
    multicast: RMC,
    last_hash_placed: bool,
    started_hashes: HashSet<H>,
    metrics: Option<Metrics<H>>,
    marker: PhantomData<&'a H>,
}

impl<
        'a,
        H: Copy + Hash,
        D: Clone + Codec + Debug + Send + Sync,
        N: DataNetwork<D>,
        MK: MultiKeychain,
        RMC: Multicast<H, SignableHash<H>>,
    > BlockSignatureAggregator<'a, H, D, N, MK, RMC>
where
    RMC::Signed: Multisigned<'a, SignableHash<H>, MK>,
{
    pub(crate) fn new(
        network: N,
        rmc: RMC,
        messages_for_rmc: mpsc::UnboundedSender<D>,
        messages_from_rmc: mpsc::UnboundedReceiver<D>,
        metrics: Option<Metrics<H>>,
    ) -> Self {
        BlockSignatureAggregator {
            messages_for_rmc,
            messages_from_rmc,
            signatures: HashMap::new(),
            hash_queue: VecDeque::new(),
            network,
            multicast: rmc,
            last_hash_placed: false,
            started_hashes: HashSet::new(),
            metrics,
            marker: PhantomData,
        }
    }

    pub(crate) async fn start_aggregation(&mut self, hash: H) {
        debug!(target: "aleph-aggregator", "Started aggregation for block hash {:?}", hash);
        if !self.started_hashes.insert(hash) {
            debug!(target: "aleph-aggregator", "Aggregation already started for block hash {:?}, exiting.", hash);
            return;
        }
        if let Some(metrics) = &self.metrics {
            metrics.report_block(hash, std::time::Instant::now(), Checkpoint::Aggregating);
        }
        self.hash_queue.push_back(hash);
        self.multicast
            .start_multicast(SignableHash::new(hash))
            .await;
    }

    pub(crate) fn notify_last_hash(&mut self) {
        self.last_hash_placed = true;
    }

    async fn wait_for_next_signature(&mut self) -> Result<(), AggregatorError> {
        loop {
            tokio::select! {
                multisigned_hash = self.multicast.next_multisigned_hash() => {
                    let hash = multisigned_hash.as_signable().hash();
                    let unchecked = multisigned_hash.into_unchecked().signature();
                    debug!(target: "aleph-aggregator", "New multisigned_hash {:?}.", unchecked);
                    self.signatures.insert(hash, unchecked);
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
                            return Err(AggregatorError::NetworkChannelClosed);
                        }
                    }
            }
        }
    }

    pub(crate) async fn next_multisigned_hash(&mut self) -> Option<(H, MK::PartialMultisignature)> {
        loop {
            trace!(target: "aleph-aggregator", "Entering next_multisigned_hash loop.");
            match self.hash_queue.front() {
                Some(hash) => {
                    if let Some(multisignature) = self.signatures.remove(hash) {
                        let hash = self
                            .hash_queue
                            .pop_front()
                            .expect("VecDeque::front() returned Some(_), qed.");
                        return Some((hash, multisignature));
                    }
                }
                None => {
                    if self.last_hash_placed {
                        debug!(target: "aleph-aggregator", "Terminating next_multisigned_hash because the last hash has been signed.");
                        return None;
                    }
                }
            }
            if self.wait_for_next_signature().await.is_err() {
                warn!(target: "aleph-aggregator", "the network channel closed");
                return None;
            }
        }
    }
}
