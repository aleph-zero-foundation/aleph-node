use futures::{Stream, StreamExt};
use log::warn;
use parity_scale_codec::Encode;
use primitives::Block;
use sp_runtime::traits::Block as _;
use substrate_prometheus_endpoint::Registry;

use super::{finality_rate::FinalityRateMetrics, timing::DefaultClock};
use crate::{
    block::ChainStatus,
    metrics::{
        best_block::BestBlockMetrics, timing::Checkpoint, transaction_pool::TransactionPoolMetrics,
        TimingBlockMetrics, LOG_TARGET,
    },
    BlockId, SubstrateChainStatus,
};

pub async fn run_metrics_service<TS: Stream<Item = TxHash> + Unpin>(
    metrics: &SloMetrics,
    transaction_pool_stream: &mut TS,
) {
    if !metrics.is_noop() {
        while let Some(tx) = transaction_pool_stream.next().await {
            metrics.report_transaction_in_pool(tx);
        }
        warn!(target: LOG_TARGET, "SLO Metrics service terminated, because the transaction pool stream ended.");
    }
}

pub type Hashing = sp_runtime::traits::HashingFor<Block>;
pub type TxHash = <Hashing as sp_runtime::traits::Hash>::Output;

#[derive(Clone)]
pub struct SloMetrics {
    timing_metrics: TimingBlockMetrics,
    finality_rate_metrics: FinalityRateMetrics,
    best_block_metrics: BestBlockMetrics,
    transaction_metrics: TransactionPoolMetrics<TxHash, DefaultClock>,
    chain_status: SubstrateChainStatus,
}

impl SloMetrics {
    pub fn new(registry: Option<&Registry>, chain_status: SubstrateChainStatus) -> Self {
        let warn_creation_failed = |name, e| warn!(target: LOG_TARGET, "Failed to register Prometheus {name} metrics: {e:?}.");
        let timing_metrics = TimingBlockMetrics::new(registry, DefaultClock).unwrap_or_else(|e| {
            warn!(
                target: LOG_TARGET,
                "Failed to register Prometheus block timing metrics: {:?}.", e
            );
            TimingBlockMetrics::Noop
        });
        let finality_rate_metrics = FinalityRateMetrics::new(registry).unwrap_or_else(|e| {
            warn!(
                target: LOG_TARGET,
                "Failed to register Prometheus finality rate metrics: {:?}.", e
            );
            FinalityRateMetrics::Noop
        });
        let best_block_metrics = BestBlockMetrics::new(registry.cloned(), chain_status.clone())
            .unwrap_or_else(|e| {
                warn_creation_failed("best block related", e);
                BestBlockMetrics::Noop
            });
        let transaction_metrics = TransactionPoolMetrics::new(registry, DefaultClock)
            .unwrap_or_else(|e| {
                warn_creation_failed("transaction pool", e);
                TransactionPoolMetrics::Noop
            });

        SloMetrics {
            timing_metrics,
            finality_rate_metrics,
            best_block_metrics,
            transaction_metrics,
            chain_status,
        }
    }

    pub fn is_noop(&self) -> bool {
        matches!(self.timing_metrics, TimingBlockMetrics::Noop)
            && matches!(self.finality_rate_metrics, FinalityRateMetrics::Noop)
            && matches!(self.best_block_metrics, BestBlockMetrics::Noop)
            && matches!(self.transaction_metrics, TransactionPoolMetrics::Noop)
    }

    pub fn timing_metrics(&self) -> &TimingBlockMetrics {
        &self.timing_metrics
    }

    pub fn report_transaction_in_pool(&self, hash: TxHash) {
        self.transaction_metrics.report_in_pool(hash);
    }

    pub fn report_block_imported(&mut self, block_id: BlockId, is_new_best: bool, own: bool) {
        self.timing_metrics
            .report_block(block_id.hash(), Checkpoint::Imported);
        if own {
            self.finality_rate_metrics
                .report_own_imported(block_id.clone());
        }
        if is_new_best {
            self.best_block_metrics
                .report_best_block_imported(block_id.clone());
        }
        if let Ok(Some(block)) = self.chain_status.block(block_id.clone()) {
            // Skip inherents - there is always exactly one, namely the timestamp inherent.
            for xt in block.extrinsics().iter().skip(1) {
                self.transaction_metrics
                    .report_in_block(xt.using_encoded(<Hashing as sp_runtime::traits::Hash>::hash));
            }
        }
    }

    pub fn report_block_finalized(&self, block_id: BlockId) {
        self.timing_metrics
            .report_block(block_id.hash(), Checkpoint::Finalized);
        self.finality_rate_metrics
            .report_finalized(block_id.clone());
        self.best_block_metrics
            .report_block_finalized(block_id.clone());
    }
}
