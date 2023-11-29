use primitives::BlockHash;

use super::{timing::DefaultClock, Checkpoint};
use crate::TimingBlockMetrics;

/// Wrapper around various block-related metrics.
#[derive(Clone)]
pub struct AllBlockMetrics {
    timing_metrics: TimingBlockMetrics<DefaultClock>,
}

impl AllBlockMetrics {
    pub fn new(timing_metrics: TimingBlockMetrics<DefaultClock>) -> Self {
        AllBlockMetrics { timing_metrics }
    }

    /// Triggers all contained block metrics.
    pub fn report_block(&self, hash: BlockHash, checkpoint: Checkpoint) {
        self.timing_metrics.report_block(hash, checkpoint);
    }
}
