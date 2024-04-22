//! Module to glue legacy and current version of the aggregator;

use std::marker::PhantomData;

use current_aleph_aggregator::NetworkError as CurrentNetworkError;
use legacy_aleph_aggregator::NetworkError as LegacyNetworkError;

use crate::{
    abft::SignatureSet,
    aleph_primitives::BlockHash,
    crypto::Signature,
    network::{
        data::{Network, SendError},
        Data,
    },
    Keychain,
};

pub type LegacyRmcNetworkData =
    legacy_aleph_aggregator::RmcNetworkData<BlockHash, Signature, SignatureSet<Signature>>;
pub type CurrentRmcNetworkData =
    current_aleph_aggregator::RmcNetworkData<BlockHash, Signature, SignatureSet<Signature>>;

pub type LegacyAggregator<N> =
    legacy_aleph_aggregator::IO<BlockHash, NetworkWrapper<LegacyRmcNetworkData, N>, Keychain>;

pub type CurrentAggregator<N> =
    current_aleph_aggregator::IO<BlockHash, NetworkWrapper<CurrentRmcNetworkData, N>, Keychain>;

enum EitherAggregator<CN, LN>
where
    LN: Network<LegacyRmcNetworkData>,
    CN: Network<CurrentRmcNetworkData>,
{
    Current(Box<CurrentAggregator<CN>>),
    Legacy(Box<LegacyAggregator<LN>>),
}

/// Wrapper on the aggregator, which is either current or legacy one. Depending on the inner variant
/// it behaves runs the legacy one or the current.
pub struct Aggregator<CN, LN>
where
    LN: Network<LegacyRmcNetworkData>,
    CN: Network<CurrentRmcNetworkData>,
{
    agg: EitherAggregator<CN, LN>,
}

impl<'a, CN, LN> Aggregator<CN, LN>
where
    LN: Network<LegacyRmcNetworkData>,
    CN: Network<CurrentRmcNetworkData>,
{
    pub fn new_legacy(multikeychain: &Keychain, rmc_network: LN) -> Self {
        let scheduler = legacy_aleph_bft_rmc::DoublingDelayScheduler::new(
            tokio::time::Duration::from_millis(500),
        );
        let rmc_handler = legacy_aleph_bft_rmc::Handler::new(multikeychain.clone());
        let rmc_service = legacy_aleph_bft_rmc::Service::new(scheduler, rmc_handler);
        let aggregator = legacy_aleph_aggregator::BlockSignatureAggregator::new();
        let aggregator_io =
            LegacyAggregator::<LN>::new(NetworkWrapper::new(rmc_network), rmc_service, aggregator);

        Self {
            agg: EitherAggregator::Legacy(Box::new(aggregator_io)),
        }
    }

    pub fn new_current(multikeychain: &Keychain, rmc_network: CN) -> Self {
        let scheduler = current_aleph_bft_rmc::DoublingDelayScheduler::new(
            tokio::time::Duration::from_millis(500),
        );
        let rmc_handler = current_aleph_bft_rmc::Handler::new(multikeychain.clone());
        let rmc_service = current_aleph_bft_rmc::Service::new(scheduler, rmc_handler);
        let aggregator = current_aleph_aggregator::BlockSignatureAggregator::new();
        let aggregator_io =
            CurrentAggregator::<CN>::new(NetworkWrapper::new(rmc_network), rmc_service, aggregator);

        Self {
            agg: EitherAggregator::Current(Box::new(aggregator_io)),
        }
    }

    pub async fn start_aggregation(&mut self, h: BlockHash) {
        match &mut self.agg {
            EitherAggregator::Current(agg) => agg.start_aggregation(h).await,
            EitherAggregator::Legacy(agg) => agg.start_aggregation(h).await,
        }
    }

    pub async fn next_multisigned_hash(&mut self) -> Option<(BlockHash, SignatureSet<Signature>)> {
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
