use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    hash::Hash,
    time::Instant,
};

use aleph_bft_rmc::{
    DoublingDelayScheduler, MultiKeychain, Multisigned, Service as RmcService, Signable,
};
use aleph_bft_types::Recipient;
use log::{debug, info, trace, warn};

use crate::{ProtocolSink, RmcNetworkData, LOG_TARGET};

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
type Rmc<H, MK, S, PMS> = RmcService<H, MK, DoublingDelayScheduler<RmcNetworkData<H, S, PMS>>>;

/// A wrapper around an `rmc::Multicast` returning the signed hashes in the order of the [`Multicast::start_multicast`] calls.
pub struct HashSignatureAggregator<H, PMS>
where
    H: Signable + Hash + Copy + Eq + Debug + Sync + Send,
{
    signatures: HashMap<H, PMS>,
    hash_queue: VecDeque<H>,
    started_hashes: HashSet<H>,
    last_change: Instant,
}

impl<H, PMS> Default for HashSignatureAggregator<H, PMS>
where
    H: Signable + Hash + Copy + Eq + Debug + Sync + Send,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<H, PMS> HashSignatureAggregator<H, PMS>
where
    H: Signable + Hash + Copy + Eq + Debug + Sync + Send,
{
    pub fn new() -> Self {
        HashSignatureAggregator {
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
        debug!(target: LOG_TARGET, "New multisigned_hash {:?}.", hash);
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
        let mut status = String::from("Hash Signature Aggregator status report: ");

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
                "front of hash queue - {:?} for - {:.2} s; ",
                hash,
                Instant::now()
                    .saturating_duration_since(self.last_change)
                    .as_secs_f64()
            ));
        }

        info!(target: LOG_TARGET, "{}", status);
    }
}

pub struct IO<H, N, MK>
where
    H: Signable + Hash + Copy + Eq + Debug + Sync + Send,
    N: ProtocolSink<RmcNetworkData<H, MK::Signature, MK::PartialMultisignature>>,
    MK: MultiKeychain,
{
    network: N,
    rmc_service: Rmc<H, MK, MK::Signature, MK::PartialMultisignature>,
    aggregator: HashSignatureAggregator<H, MK::PartialMultisignature>,
    multisigned_events: VecDeque<Multisigned<H, MK>>,
}

impl<H, N, MK> IO<H, N, MK>
where
    H: Signable + Hash + Copy + Eq + Debug + Sync + Send,
    N: ProtocolSink<RmcNetworkData<H, MK::Signature, MK::PartialMultisignature>>,
    MK: MultiKeychain,
{
    pub fn new(
        network: N,
        rmc_service: Rmc<H, MK, MK::Signature, MK::PartialMultisignature>,
        aggregator: HashSignatureAggregator<H, MK::PartialMultisignature>,
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
        debug!(target: LOG_TARGET, "Started aggregation for hash {:?}", hash);
        if let Err(AggregatorError::DuplicateHash) = self.aggregator.on_start(hash) {
            debug!(target: LOG_TARGET, "Aggregation already started for hash {:?}, ignoring.", hash);
            return;
        }
        if let Some(multisigned) = self.rmc_service.start_rmc(hash) {
            self.multisigned_events.push_back(multisigned);
        }
    }

    async fn wait_for_next_signature(&mut self) -> IOResult {
        loop {
            if let Some(multisigned) = self.multisigned_events.pop_front() {
                let unchecked = multisigned.into_unchecked();
                let signature = unchecked.signature();
                self.aggregator
                    .on_multisigned_hash(unchecked.into_signable(), signature);
                return Ok(());
            }
            tokio::select! {
                message_from_rmc = self.rmc_service.next_message() => {
                    trace!(target: LOG_TARGET, "Our rmc message {:?}.", message_from_rmc);
                    if let Err(e) = self.network.send(message_from_rmc, Recipient::Everyone) {
                        warn!(target: LOG_TARGET, "failed broadcasting a message from rmc: {:?}", e);
                    }
                }
                message_from_network = self.network.next() =>  {
                    // In case the network is down we can terminate (?).
                    let message = message_from_network.ok_or(IOError::NetworkChannelClosed)?;
                    trace!(target: LOG_TARGET, "Received message for rmc: {:?}", message);
                    if let Some(multisigned) = self.rmc_service.process_message(message) {
                        self.multisigned_events.push_back(multisigned);
                    }
                }
            }
        }
    }

    pub async fn next_multisigned_hash(&mut self) -> Option<(H, MK::PartialMultisignature)> {
        loop {
            trace!(target: LOG_TARGET, "Entering next_multisigned_hash loop.");
            match self.aggregator.try_pop_hash() {
                Ok(res) => {
                    return Some(res);
                }
                Err(AggregatorError::NoHashFound) => { /* ignored */ }
                Err(AggregatorError::DuplicateHash) => {
                    warn!(
                        target: LOG_TARGET,
                        "Unexpected aggregator exception in IO: DuplicateHash",
                    )
                }
            }

            if self.wait_for_next_signature().await.is_err() {
                warn!(target: LOG_TARGET, "the network channel closed");
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

    use crate::aggregator::{AggregatorError, HashSignatureAggregator};

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

    fn build_aggregator() -> HashSignatureAggregator<MockHash, TestMultisignature> {
        HashSignatureAggregator::new()
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
