use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    time::Instant,
};

use aleph_bft_types::Recipient;
use codec::Codec;
use futures::{channel::mpsc, StreamExt};
use log::{debug, error, info, trace, warn};

use crate::{
    multicast::{Hash, Multicast, SignableHash},
    Metrics, ProtocolSink,
};

#[derive(Debug, PartialEq, Eq)]
pub enum AggregatorError {
    NoHashFound,
    DuplicateHash,
}

pub enum IOError {
    NetworkChannelClosed,
}

pub type AggregatorResult<R> = Result<R, AggregatorError>;
pub type IOResult = Result<(), IOError>;

/// A wrapper around an `rmc::Multicast` returning the signed hashes in the order of the [`Multicast::start_multicast`] calls.
pub struct BlockSignatureAggregator<H: Hash + Copy, PMS, M: Metrics<H>> {
    signatures: HashMap<H, PMS>,
    hash_queue: VecDeque<H>,
    started_hashes: HashSet<H>,
    metrics: Option<M>,
    last_change: Instant,
}

impl<H: Copy + Hash, PMS, M: Metrics<H>> BlockSignatureAggregator<H, PMS, M> {
    pub fn new(metrics: Option<M>) -> Self {
        BlockSignatureAggregator {
            signatures: HashMap::new(),
            hash_queue: VecDeque::new(),
            started_hashes: HashSet::new(),
            last_change: Instant::now(),
            metrics,
        }
    }

    fn on_start(&mut self, hash: H) -> AggregatorResult<()> {
        if !self.started_hashes.insert(hash) {
            return Err(AggregatorError::DuplicateHash);
        }
        if let Some(metrics) = &mut self.metrics {
            metrics.report_aggregation_complete(hash);
        }
        if self.hash_queue.is_empty() {
            self.last_change = Instant::now();
        }
        self.hash_queue.push_back(hash);

        Ok(())
    }

    fn on_multisigned_hash(&mut self, hash: H, signature: PMS) {
        debug!(target: "aleph-aggregator", "New multisigned_hash {:?}.", hash);
        self.signatures.insert(hash, signature);
    }

    fn try_pop_hash(&mut self) -> AggregatorResult<(H, PMS)> {
        match self.hash_queue.pop_front() {
            Some(hash) => {
                if let Some(multisignature) = self.signatures.remove(&hash) {
                    self.last_change = Instant::now();
                    Ok((hash, multisignature))
                } else {
                    self.hash_queue.push_front(hash);
                    Err(AggregatorError::NoHashFound)
                }
            }
            None => Err(AggregatorError::NoHashFound),
        }
    }

    pub fn status_report(&self) {
        let mut status = String::from("Block Signature Aggregator status report: ");

        status.push_str(&format!(
            "started hashes - {:?}; ",
            self.started_hashes.len()
        ));

        status.push_str(&format!(
            "collected signatures - {:?}; ",
            self.signatures.len()
        ));

        status.push_str(&format!("hashes in queue - {:?}; ", self.hash_queue.len()));

        if let Some(hash) = self.hash_queue.front() {
            status.push_str(&format!(
                "front of hash queue - {} for - {:.2} s; ",
                hash,
                Instant::now()
                    .saturating_duration_since(self.last_change)
                    .as_secs_f64()
            ));
        }

        info!(target: "aleph-aggregator", "{}", status);
    }
}

pub struct IO<
    H: Hash + Copy,
    D: Clone + Codec + Debug + Send + Sync + 'static,
    N: ProtocolSink<D>,
    PMS,
    RMC: Multicast<H, PMS>,
    M: Metrics<H>,
> {
    messages_for_rmc: mpsc::UnboundedSender<D>,
    messages_from_rmc: mpsc::UnboundedReceiver<D>,
    network: N,
    multicast: RMC,
    aggregator: BlockSignatureAggregator<H, PMS, M>,
}

impl<
        H: Copy + Hash,
        D: Clone + Codec + Debug + Send + Sync,
        N: ProtocolSink<D>,
        PMS,
        RMC: Multicast<H, PMS>,
        M: Metrics<H>,
    > IO<H, D, N, PMS, RMC, M>
{
    pub fn new(
        messages_for_rmc: mpsc::UnboundedSender<D>,
        messages_from_rmc: mpsc::UnboundedReceiver<D>,
        network: N,
        multicast: RMC,
        aggregator: BlockSignatureAggregator<H, PMS, M>,
    ) -> Self {
        IO {
            messages_for_rmc,
            messages_from_rmc,
            network,
            multicast,
            aggregator,
        }
    }

    pub fn status_report(&self) {
        self.aggregator.status_report()
    }

    pub async fn start_aggregation(&mut self, hash: H) {
        debug!(target: "aleph-aggregator", "Started aggregation for block hash {:?}", hash);
        if let Err(AggregatorError::DuplicateHash) = self.aggregator.on_start(hash) {
            debug!(target: "aleph-aggregator", "Aggregation already started for block hash {:?}, ignoring.", hash);
            return;
        }
        self.multicast
            .start_multicast(SignableHash::new(hash))
            .await;
    }

    async fn wait_for_next_signature(&mut self) -> IOResult {
        loop {
            tokio::select! {
                (hash, signature) = self.multicast.next_signed_pair() => {
                    self.aggregator.on_multisigned_hash(hash, signature);
                    return Ok(());
                }
                message_from_rmc = self.messages_from_rmc.next() => {
                    trace!(target: "aleph-aggregator", "Our rmc message {:?}.", message_from_rmc);
                    match message_from_rmc {
                        Some(message_from_rmc) => {
                            if let Err(e) = self.network.send(message_from_rmc, Recipient::Everyone) {
                                error!(target: "aleph-aggregator", "error sending message from rmc.\n{:?}", e);
                            }
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

    pub async fn next_multisigned_hash(&mut self) -> Option<(H, PMS)> {
        loop {
            trace!(target: "aleph-aggregator", "Entering next_multisigned_hash loop.");
            match self.aggregator.try_pop_hash() {
                Ok(res) => {
                    return Some(res);
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

#[cfg(test)]
mod tests {
    use std::{
        fmt::{Debug, Display, Formatter},
        hash::Hash,
    };

    use codec::{Decode, Encode};

    use crate::{
        aggregator::{AggregatorError, BlockSignatureAggregator},
        Metrics,
    };

    #[derive(Hash, PartialEq, Eq, Clone, Copy, Encode, Decode, Debug)]
    struct MockHash(pub [u8; 32]);

    impl AsRef<[u8]> for MockHash {
        fn as_ref(&self) -> &[u8] {
            &self.0
        }
    }

    impl Display for MockHash {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            std::fmt::Debug::fmt(&self.0, f)
        }
    }
    type TestMultisignature = usize;
    const TEST_SIGNATURE: TestMultisignature = 42;

    struct MockMetrics;
    impl Metrics<MockHash> for MockMetrics {
        fn report_aggregation_complete(&mut self, _h: MockHash) {}
    }

    fn build_aggregator() -> BlockSignatureAggregator<MockHash, TestMultisignature, MockMetrics> {
        BlockSignatureAggregator::new(None)
    }

    fn build_hash(b0: u8) -> MockHash {
        let mut bytes = [0u8; 32];
        bytes[0] = b0;
        MockHash(bytes)
    }

    #[test]
    fn returns_with_matching_multisigned_hash() {
        let mut aggregator = build_aggregator();
        let res = aggregator.on_start(build_hash(0));
        assert!(res.is_ok());

        aggregator.on_multisigned_hash(build_hash(0), TEST_SIGNATURE);

        let res = aggregator.try_pop_hash();
        assert!(res.is_ok());
    }

    #[test]
    fn doesnt_return_without_matching_multisigned_hash() {
        let mut aggregator = build_aggregator();
        let res = aggregator.on_start(build_hash(0));
        assert!(res.is_ok());

        aggregator.on_multisigned_hash(build_hash(1), TEST_SIGNATURE);

        let res = aggregator.try_pop_hash();
        assert_eq!(res, Err(AggregatorError::NoHashFound));
    }
}
