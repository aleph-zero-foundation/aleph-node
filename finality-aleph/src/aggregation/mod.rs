//! Module to glue legacy and current version of the aggregator;

use std::{fmt::Debug, hash::Hash, marker::PhantomData, time::Instant};

use current_aleph_aggregator::NetworkError as CurrentNetworkError;
use legacy_aleph_aggregator::NetworkError as LegacyNetworkError;
use sp_runtime::traits::Block;

use crate::{
    abft::SignatureSet,
    crypto::Signature,
    metrics::Checkpoint,
    mpsc,
    network::{
        data::{Network, SendError},
        Data,
    },
    Keychain, Metrics,
};

pub type LegacyRmcNetworkData<B> =
    legacy_aleph_aggregator::RmcNetworkData<<B as Block>::Hash, Signature, SignatureSet<Signature>>;
pub type CurrentRmcNetworkData<B> = current_aleph_aggregator::RmcNetworkData<
    <B as Block>::Hash,
    Signature,
    SignatureSet<Signature>,
>;

pub type LegacySignableBlockHash<B> = legacy_aleph_aggregator::SignableHash<<B as Block>::Hash>;
pub type LegacyRmc<'a, B> =
    legacy_aleph_bft_rmc::ReliableMulticast<'a, LegacySignableBlockHash<B>, Keychain>;
pub type LegacyAggregator<'a, B, N> = legacy_aleph_aggregator::IO<
    <B as Block>::Hash,
    LegacyRmcNetworkData<B>,
    NetworkWrapper<LegacyRmcNetworkData<B>, N>,
    SignatureSet<Signature>,
    LegacyRmc<'a, B>,
    Metrics<<B as Block>::Hash>,
>;

pub type CurrentSignableBlockHash<B> = current_aleph_aggregator::SignableHash<<B as Block>::Hash>;
pub type CurrentRmc<'a, B> =
    current_aleph_bft_rmc::ReliableMulticast<'a, CurrentSignableBlockHash<B>, Keychain>;
pub type CurrentAggregator<'a, B, N> = current_aleph_aggregator::IO<
    <B as Block>::Hash,
    CurrentRmcNetworkData<B>,
    NetworkWrapper<CurrentRmcNetworkData<B>, N>,
    SignatureSet<Signature>,
    CurrentRmc<'a, B>,
    Metrics<<B as Block>::Hash>,
>;

enum EitherAggregator<'a, B, CN, LN>
where
    B: Block,
    LN: Network<LegacyRmcNetworkData<B>>,
    CN: Network<CurrentRmcNetworkData<B>>,
    <B as Block>::Hash: AsRef<[u8]>,
{
    Current(CurrentAggregator<'a, B, CN>),
    Legacy(LegacyAggregator<'a, B, LN>),
}

/// Wrapper on the aggregator, which is either current or legacy one. Depending on the inner variant
/// it behaves runs the legacy one or the current.
pub struct Aggregator<'a, B, CN, LN>
where
    B: Block,
    LN: Network<LegacyRmcNetworkData<B>>,
    CN: Network<CurrentRmcNetworkData<B>>,
    <B as Block>::Hash: AsRef<[u8]>,
{
    agg: EitherAggregator<'a, B, CN, LN>,
}

impl<'a, B, CN, LN> Aggregator<'a, B, CN, LN>
where
    B: Block,
    LN: Network<LegacyRmcNetworkData<B>>,
    CN: Network<CurrentRmcNetworkData<B>>,
    <B as Block>::Hash: AsRef<[u8]>,
{
    pub fn new_legacy(
        multikeychain: &'a Keychain,
        rmc_network: LN,
        metrics: Option<Metrics<<B as Block>::Hash>>,
    ) -> Self {
        let (messages_for_rmc, messages_from_network) = mpsc::unbounded();
        let (messages_for_network, messages_from_rmc) = mpsc::unbounded();
        let scheduler = legacy_aleph_bft_rmc::DoublingDelayScheduler::new(
            tokio::time::Duration::from_millis(500),
        );
        let rmc = legacy_aleph_bft_rmc::ReliableMulticast::new(
            messages_from_network,
            messages_for_network,
            multikeychain,
            legacy_aleph_bft::Keychain::node_count(multikeychain),
            scheduler,
        );
        let aggregator = legacy_aleph_aggregator::BlockSignatureAggregator::new(metrics);
        let aggregator_io = LegacyAggregator::<B, LN>::new(
            messages_for_rmc,
            messages_from_rmc,
            NetworkWrapper::new(rmc_network),
            rmc,
            aggregator,
        );

        Self {
            agg: EitherAggregator::Legacy(aggregator_io),
        }
    }

    pub fn new_current(
        multikeychain: &'a Keychain,
        rmc_network: CN,
        metrics: Option<Metrics<<B as Block>::Hash>>,
    ) -> Self {
        let (messages_for_rmc, messages_from_network) = mpsc::unbounded();
        let (messages_for_network, messages_from_rmc) = mpsc::unbounded();
        let scheduler = current_aleph_bft_rmc::DoublingDelayScheduler::new(
            tokio::time::Duration::from_millis(500),
        );
        let rmc = current_aleph_bft_rmc::ReliableMulticast::new(
            messages_from_network,
            messages_for_network,
            multikeychain,
            current_aleph_bft::Keychain::node_count(multikeychain),
            scheduler,
        );
        let aggregator = current_aleph_aggregator::BlockSignatureAggregator::new(metrics);
        let aggregator_io = CurrentAggregator::<B, CN>::new(
            messages_for_rmc,
            messages_from_rmc,
            NetworkWrapper::new(rmc_network),
            rmc,
            aggregator,
        );

        Self {
            agg: EitherAggregator::Current(aggregator_io),
        }
    }

    pub async fn start_aggregation(&mut self, h: <B as Block>::Hash) {
        match &mut self.agg {
            EitherAggregator::Current(agg) => agg.start_aggregation(h).await,
            EitherAggregator::Legacy(agg) => agg.start_aggregation(h).await,
        }
    }

    pub async fn next_multisigned_hash(
        &mut self,
    ) -> Option<(<B as Block>::Hash, SignatureSet<Signature>)> {
        match &mut self.agg {
            EitherAggregator::Current(agg) => agg.next_multisigned_hash().await,
            EitherAggregator::Legacy(agg) => agg.next_multisigned_hash().await,
        }
    }

    pub fn status_report(&self) {
        match &self.agg {
            EitherAggregator::Current(agg) => agg.status_report(),
            EitherAggregator::Legacy(agg) => agg.status_report(),
        }
    }
}

pub struct NetworkWrapper<D: Data, N: Network<D>>(N, PhantomData<D>);

impl<D: Data, N: Network<D>> NetworkWrapper<D, N> {
    pub fn new(network: N) -> Self {
        Self(network, PhantomData)
    }
}

impl<H: Debug + Hash + Eq + Debug + Copy> legacy_aleph_aggregator::Metrics<H> for Metrics<H> {
    fn report_aggregation_complete(&mut self, h: H) {
        self.report_block(h, Instant::now(), Checkpoint::Aggregating);
    }
}

impl<H: Debug + Hash + Eq + Debug + Copy> current_aleph_aggregator::Metrics<H> for Metrics<H> {
    fn report_aggregation_complete(&mut self, h: H) {
        self.report_block(h, Instant::now(), Checkpoint::Aggregating);
    }
}

#[async_trait::async_trait]
impl<T, D> legacy_aleph_aggregator::ProtocolSink<D> for NetworkWrapper<D, T>
where
    T: Network<D>,
    D: Data,
{
    async fn next(&mut self) -> Option<D> {
        self.0.next().await
    }

    fn send(
        &self,
        data: D,
        recipient: legacy_aleph_bft::Recipient,
    ) -> Result<(), LegacyNetworkError> {
        self.0.send(data, recipient.into()).map_err(|e| match e {
            SendError::SendFailed => LegacyNetworkError::SendFail,
        })
    }
}

#[async_trait::async_trait]
impl<T, D> current_aleph_aggregator::ProtocolSink<D> for NetworkWrapper<D, T>
where
    T: Network<D>,
    D: Data,
{
    async fn next(&mut self) -> Option<D> {
        self.0.next().await
    }

    fn send(
        &self,
        data: D,
        recipient: current_aleph_bft::Recipient,
    ) -> Result<(), CurrentNetworkError> {
        self.0.send(data, recipient.into()).map_err(|e| match e {
            SendError::SendFailed => CurrentNetworkError::SendFail,
        })
    }
}
