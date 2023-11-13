use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    time::Instant,
};

use aleph_bft_rmc::{DoublingDelayScheduler, MultiKeychain, Multisigned, Service as RmcService};
use aleph_bft_types::Recipient;
use log::{debug, info, trace, warn};

use crate::{Hash, ProtocolSink, RmcNetworkData, SignableHash};

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
type Rmc<H, MK, S, PMS> =
    RmcService<SignableHash<H>, MK, DoublingDelayScheduler<RmcNetworkData<H, S, PMS>>>;

/// A wrapper around an `rmc::Multicast` returning the signed hashes in the order of the [`Multicast::start_multicast`] calls.
pub struct BlockSignatureAggregator<H: Hash + Copy, PMS> {
    signatures: HashMap<H, PMS>,
    hash_queue: VecDeque<H>,
    started_hashes: HashSet<H>,
    last_change: Instant,
}

impl<H: Copy + Hash, PMS> Default for BlockSignatureAggregator<H, PMS> {
    fn default() -> Self {
        Self::new()
    }
}

impl<H: Copy + Hash, PMS> BlockSignatureAggregator<H, PMS> {
    pub fn new() -> Self {
        BlockSignatureAggregator {
            signatures: HashMap::new(),
            hash_queue: VecDeque::new(),
            started_hashes: HashSet::new(),
            last_change: Instant::now(),
        }
    }

    fn on_start(&mut self, hash: H) -> AggregatorResult<()> {
        if !self.started_hashes.insert(hash) {
            return Err(AggregatorError::DuplicateHash);
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
    N: ProtocolSink<RmcNetworkData<H, MK::Signature, MK::PartialMultisignature>>,
    MK: MultiKeychain,
> {
    network: N,
    rmc_service: Rmc<H, MK, MK::Signature, MK::PartialMultisignature>,
    aggregator: BlockSignatureAggregator<H, MK::PartialMultisignature>,
    multisigned_events: VecDeque<Multisigned<SignableHash<H>, MK>>,
}

impl<
        H: Copy + Hash,
        N: ProtocolSink<RmcNetworkData<H, MK::Signature, MK::PartialMultisignature>>,
        MK: MultiKeychain,
    > IO<H, N, MK>
{
    pub fn new(
        network: N,
        rmc_service: Rmc<H, MK, MK::Signature, MK::PartialMultisignature>,
        aggregator: BlockSignatureAggregator<H, MK::PartialMultisignature>,
    ) -> Self {
        IO {
            network,
            rmc_service,
            aggregator,
            multisigned_events: VecDeque::new(),
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
        if let Some(multisigned) = self.rmc_service.start_rmc(SignableHash::new(hash)) {
            self.multisigned_events.push_back(multisigned);
        }
    }

    async fn wait_for_next_signature(&mut self) -> IOResult {
        loop {
            if let Some(multisigned) = self.multisigned_events.pop_front() {
                let unchecked = multisigned.into_unchecked();
                let signature = unchecked.signature();
                self.aggregator
                    .on_multisigned_hash(unchecked.into_signable().get_hash(), signature);
                return Ok(());
            }
            tokio::select! {
                message_from_rmc = self.rmc_service.next_message() => {
                    trace!(target: "aleph-aggregator", "Our rmc message {:?}.", message_from_rmc);
                    if let Err(e) = self.network.send(message_from_rmc, Recipient::Everyone) {
                        warn!(target: "aleph-aggregator", "failed broadcasting a message from rmc: {:?}", e);
                    }
                }
                message_from_network = self.network.next() => match message_from_network {
                    Some(message) => {
                        trace!(target: "aleph-aggregator", "Received message for rmc: {:?}", message);
                        if let Some(multisigned) = self.rmc_service.process_message(message) {
                            self.multisigned_events.push_back(multisigned);
                        }
                    },
                    None => {
                        // In case the network is down we can terminate (?).
                        return Err(IOError::NetworkChannelClosed);
                    }
                }
            }
        }
    }

    pub async fn next_multisigned_hash(&mut self) -> Option<(H, MK::PartialMultisignature)> {
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

    use parity_scale_codec::{Decode, Encode};

    use crate::aggregator::{AggregatorError, BlockSignatureAggregator};

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

    fn build_aggregator() -> BlockSignatureAggregator<MockHash, TestMultisignature> {
        BlockSignatureAggregator::new()
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
