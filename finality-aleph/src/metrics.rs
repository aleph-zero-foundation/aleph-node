use std::{
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    num::NonZeroUsize,
    time::{Duration, Instant},
};

use log::{trace, warn};
use lru::LruCache;
use parking_lot::Mutex;
use sc_service::Arc;
use substrate_prometheus_endpoint::{
    exponential_buckets, prometheus, register, Histogram, HistogramOpts, PrometheusError, Registry,
};

use crate::{aleph_primitives::BlockHash, Display};

// How many entries (block hash + timestamp) we keep in memory per one checkpoint type.
// Each entry takes 32B (Hash) + 16B (Instant), so a limit of 5000 gives ~234kB (per checkpoint).
// Notice that some issues like finalization stall may lead to incomplete metrics
// (e.g. when the gap between checkpoints for a block grows over `MAX_BLOCKS_PER_CHECKPOINT`).
const MAX_BLOCKS_PER_CHECKPOINT: usize = 5000;

const LOG_TARGET: &str = "aleph-metrics";

#[derive(Clone)]
pub enum TimingBlockMetrics {
    Prometheus {
        time_since_prev_checkpoint: HashMap<Checkpoint, Histogram>,
        imported_to_finalized: Histogram,
        starts: Arc<Mutex<HashMap<Checkpoint, LruCache<BlockHash, Instant>>>>,
    },
    Noop,
}

impl TimingBlockMetrics {
    pub fn new(registry: Option<&Registry>) -> Result<Self, PrometheusError> {
        use Checkpoint::*;
        let keys = [Importing, Imported, Proposed, Ordered, Finalized];
        let target_time_since_prev_checkpoint = HashMap::from([
            (Imported, 100.0),
            (Proposed, 1000.0),
            (Ordered, 1000.0),
            (Finalized, 150.0),
        ]);

        const BUCKETS_FACTOR: f64 = 1.5;

        let registry = match registry {
            None => return Ok(Self::Noop),
            Some(registry) => registry,
        };

        let mut time_since_prev_checkpoint = HashMap::new();

        for key in keys[1..].iter() {
            let target = target_time_since_prev_checkpoint
                .get(key)
                .copied()
                .expect("Target times initialized");
            time_since_prev_checkpoint.insert(
                *key,
                register(
                    Histogram::with_opts(
                        HistogramOpts::new(
                            format!("aleph_timing_{}", key.to_string().to_ascii_lowercase()),
                            "no help",
                        )
                        .buckets(exponential_buckets_two_sided(
                            target,
                            BUCKETS_FACTOR,
                            4,
                            6,
                        )?),
                    )?,
                    registry,
                )?,
            );
        }

        Ok(Self::Prometheus {
            time_since_prev_checkpoint,
            imported_to_finalized: register(
                Histogram::with_opts(
                    HistogramOpts::new("aleph_timing_imported_to_finalized", "no help")
                        .buckets(exponential_buckets_two_sided(2000.0, BUCKETS_FACTOR, 4, 6)?),
                )?,
                registry,
            )?,
            starts: Arc::new(Mutex::new(
                keys.iter()
                    .map(|k| {
                        (
                            *k,
                            LruCache::new(NonZeroUsize::new(MAX_BLOCKS_PER_CHECKPOINT).unwrap()),
                        )
                    })
                    .collect(),
            )),
        })
    }

    pub fn noop() -> Self {
        Self::Noop
    }

    pub fn report_block_if_not_present(
        &self,
        hash: BlockHash,
        checkpoint_time: Instant,
        checkpoint_type: Checkpoint,
    ) {
        let starts = match self {
            TimingBlockMetrics::Noop => return,
            TimingBlockMetrics::Prometheus { starts, .. } => starts,
        };
        if !starts
            .lock()
            .get_mut(&checkpoint_type)
            .expect("All checkpoint types were initialized")
            .contains(&hash)
        {
            self.report_block(hash, checkpoint_time, checkpoint_type);
        }
    }

    pub fn report_block(
        &self,
        hash: BlockHash,
        checkpoint_time: Instant,
        checkpoint_type: Checkpoint,
    ) {
        trace!(
            target: LOG_TARGET,
            "Reporting block stage: {:?} (hash: {:?}, at: {:?}",
            checkpoint_type,
            hash,
            checkpoint_time
        );
        let (time_since_prev_checkpoint, imported_to_finalized, starts) = match self {
            TimingBlockMetrics::Noop => return,
            TimingBlockMetrics::Prometheus {
                time_since_prev_checkpoint,
                imported_to_finalized,
                starts,
            } => (time_since_prev_checkpoint, imported_to_finalized, starts),
        };

        let starts = &mut *starts.lock();
        starts.entry(checkpoint_type).and_modify(|starts| {
            starts.put(hash, checkpoint_time);
        });

        if let Some(prev_checkpoint_type) = checkpoint_type.prev() {
            if let Some(start) = starts
                .get_mut(&prev_checkpoint_type)
                .expect("All checkpoint types were initialized")
                .get(&hash)
            {
                let duration = checkpoint_time
                    .checked_duration_since(*start)
                    .unwrap_or_else(|| {
                        Self::warn_about_monotonicity_violation(
                            *start,
                            checkpoint_time,
                            checkpoint_type,
                            hash,
                        );
                        Duration::new(0, 0)
                    });
                time_since_prev_checkpoint
                    .get(&checkpoint_type)
                    .expect("All checkpoint types were initialized")
                    .observe(duration.as_secs_f64() * 1000.);
            }
        }
        if checkpoint_type == Checkpoint::Finalized {
            if let Some(start) = starts
                .get_mut(&Checkpoint::Imported)
                .expect("All checkpoint types were initialized")
                .get(&hash)
            {
                let duration = checkpoint_time
                    .checked_duration_since(*start)
                    .unwrap_or_else(|| {
                        Self::warn_about_monotonicity_violation(
                            *start,
                            checkpoint_time,
                            checkpoint_type,
                            hash,
                        );
                        Duration::new(0, 0)
                    });
                imported_to_finalized.observe(duration.as_secs_f64() * 1000.);
            }
        }
    }

    fn warn_about_monotonicity_violation(
        start: Instant,
        checkpoint_time: Instant,
        checkpoint_type: Checkpoint,
        hash: BlockHash,
    ) {
        warn!(
            target: LOG_TARGET,
            "Earlier metrics time {:?} is later that current one \
        {:?}. Checkpoint type {:?}, block: {:?}",
            start,
            checkpoint_time,
            checkpoint_type,
            hash
        );
    }
}

#[derive(Clone, Copy, Debug, Display, Hash, PartialEq, Eq)]
pub enum Checkpoint {
    Importing,
    Imported,
    Proposed,
    Ordered,
    Finalized,
}

impl Checkpoint {
    fn prev(&self) -> Option<Checkpoint> {
        use Checkpoint::*;
        match self {
            Importing => None,
            Imported => Some(Importing),
            Proposed => Some(Imported),
            Ordered => Some(Proposed),
            Finalized => Some(Ordered),
        }
    }
}

/// Create `count_below` + 1 + `count_above` buckets, where (`count_below` + 1)th bucket
/// has an upper bound `start`. The buckets are exponentially distributed with a factor `factor`.
fn exponential_buckets_two_sided(
    start: f64,
    factor: f64,
    count_below: usize,
    count_above: usize,
) -> prometheus::Result<Vec<f64>> {
    let mut strictly_smaller =
        exponential_buckets(start / factor.powi(count_below as i32), factor, count_below)?;
    let mut greater_than_or_equal = exponential_buckets(start, factor, 1 + count_above)?;
    if strictly_smaller.last().is_some()
        && strictly_smaller.last().unwrap()
            >= greater_than_or_equal
                .first()
                .expect("There is at least one checkpoint")
    {
        return Err(prometheus::Error::Msg(
            "Floating point arithmetic error causing incorrect buckets, try larger factor or smaller count_below"
                .to_string(),
        ));
    }
    strictly_smaller.append(&mut greater_than_or_equal);
    Ok(strictly_smaller)
}

#[cfg(test)]
mod tests {
    use std::cmp::min;

    use Checkpoint::*;

    use super::*;

    fn register_prometheus_metrics_with_dummy_registry() -> TimingBlockMetrics {
        TimingBlockMetrics::new(Some(&Registry::new())).unwrap()
    }

    fn starts_for(m: &TimingBlockMetrics, c: Checkpoint) -> usize {
        match &m {
            TimingBlockMetrics::Prometheus { starts, .. } => starts.lock().get(&c).unwrap().len(),
            _ => 0,
        }
    }

    fn check_reporting_with_memory_excess(metrics: &TimingBlockMetrics, checkpoint: Checkpoint) {
        for i in 1..(MAX_BLOCKS_PER_CHECKPOINT + 10) {
            metrics.report_block(BlockHash::random(), Instant::now(), checkpoint);
            assert_eq!(
                min(i, MAX_BLOCKS_PER_CHECKPOINT),
                starts_for(metrics, checkpoint)
            )
        }
    }

    #[test]
    fn noop_metrics() {
        let m = TimingBlockMetrics::noop();
        m.report_block(BlockHash::random(), Instant::now(), Ordered);
        assert!(matches!(m, TimingBlockMetrics::Noop));
    }

    #[test]
    fn should_keep_entries_up_to_defined_limit() {
        let m = register_prometheus_metrics_with_dummy_registry();
        check_reporting_with_memory_excess(&m, Ordered);
    }

    #[test]
    fn should_manage_space_for_checkpoints_independently() {
        let m = register_prometheus_metrics_with_dummy_registry();
        check_reporting_with_memory_excess(&m, Ordered);
        check_reporting_with_memory_excess(&m, Imported);
    }

    #[test]
    fn given_not_monotonic_clock_when_report_block_is_called_repeatedly_code_does_not_panic() {
        let metrics = register_prometheus_metrics_with_dummy_registry();
        let earlier_timestamp = Instant::now();
        let later_timestamp = earlier_timestamp + Duration::new(0, 5);
        let hash = BlockHash::random();
        metrics.report_block(hash, later_timestamp, Proposed);
        metrics.report_block(hash, earlier_timestamp, Ordered);
    }

    #[test]
    fn test_report_block_if_not_present() {
        let metrics = register_prometheus_metrics_with_dummy_registry();
        let earlier_timestamp = Instant::now();
        let later_timestamp = earlier_timestamp + Duration::new(0, 5);
        let hash = BlockHash::random();

        metrics.report_block(hash, earlier_timestamp, Proposed);
        metrics.report_block_if_not_present(hash, later_timestamp, Proposed);

        let timestamp = match &metrics {
            TimingBlockMetrics::Prometheus { starts, .. } => starts
                .lock()
                .get_mut(&Proposed)
                .unwrap()
                .get(&hash)
                .cloned(),
            _ => None,
        };
        assert_eq!(timestamp, Some(earlier_timestamp));
    }
}
