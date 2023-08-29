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
use substrate_prometheus_endpoint::{register, Gauge, PrometheusError, Registry, U64};

use crate::aleph_primitives::BlockHash;

// How many entries (block hash + timestamp) we keep in memory per one checkpoint type.
// Each entry takes 32B (Hash) + 16B (Instant), so a limit of 5000 gives ~234kB (per checkpoint).
// Notice that some issues like finalization stall may lead to incomplete metrics
// (e.g. when the gap between checkpoints for a block grows over `MAX_BLOCKS_PER_CHECKPOINT`).
const MAX_BLOCKS_PER_CHECKPOINT: usize = 5000;

const LOG_TARGET: &str = "aleph-metrics";

struct Inner {
    prev: HashMap<Checkpoint, Checkpoint>,
    gauges: HashMap<Checkpoint, Gauge<U64>>,
    starts: HashMap<Checkpoint, LruCache<BlockHash, Instant>>,
}

impl Inner {
    fn new(registry: &Registry) -> Result<Self, PrometheusError> {
        use Checkpoint::*;
        let keys = [
            Importing,
            Imported,
            Ordering,
            Ordered,
            Aggregating,
            Finalized,
        ];
        let prev: HashMap<_, _> = keys[1..]
            .iter()
            .cloned()
            .zip(keys.iter().cloned())
            .collect();

        let mut gauges = HashMap::new();
        for key in keys.iter() {
            gauges.insert(
                *key,
                register(Gauge::new(format!("aleph_{key:?}"), "no help")?, registry)?,
            );
        }

        Ok(Self {
            prev,
            gauges,
            starts: keys
                .iter()
                .map(|k| {
                    (
                        *k,
                        LruCache::new(NonZeroUsize::new(MAX_BLOCKS_PER_CHECKPOINT).unwrap()),
                    )
                })
                .collect(),
        })
    }

    fn report_block(
        &mut self,
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

        self.starts.entry(checkpoint_type).and_modify(|starts| {
            starts.put(hash, checkpoint_time);
        });

        if let Some(prev_checkpoint_type) = self.prev.get(&checkpoint_type) {
            if let Some(start) = self
                .starts
                .get_mut(prev_checkpoint_type)
                .expect("All checkpoint types were initialized")
                .get(&hash)
            {
                let duration = match checkpoint_time.checked_duration_since(*start) {
                    Some(duration) => duration,
                    None => {
                        warn!(
                            target: LOG_TARGET,
                            "Earlier metrics time {:?} is later that current one \
                        {:?}. Checkpoint type {:?}, block: {:?}",
                            *start,
                            checkpoint_time,
                            checkpoint_type,
                            hash
                        );
                        Duration::new(0, 0)
                    }
                };
                self.gauges
                    .get(&checkpoint_type)
                    .expect("All checkpoint types were initialized")
                    .set(duration.as_millis() as u64);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Checkpoint {
    Importing,
    Imported,
    Ordering,
    Ordered,
    Aggregating,
    Finalized,
}

/// TODO(A0-3009): Replace this whole thing.
#[derive(Clone)]
pub struct BlockMetrics {
    inner: Option<Arc<Mutex<Inner>>>,
}

impl BlockMetrics {
    pub fn noop() -> Self {
        Self { inner: None }
    }

    pub fn new(registry: &Registry) -> Result<Self, PrometheusError> {
        let inner = Some(Arc::new(Mutex::new(Inner::new(registry)?)));

        Ok(Self { inner })
    }

    pub fn report_block(
        &self,
        hash: BlockHash,
        checkpoint_time: Instant,
        checkpoint_type: Checkpoint,
    ) {
        if let Some(inner) = &self.inner {
            inner
                .lock()
                .report_block(hash, checkpoint_time, checkpoint_type);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::min;

    use super::*;

    fn register_dummy_metrics() -> BlockMetrics {
        BlockMetrics::new(&Registry::new()).unwrap()
    }

    fn starts_for(m: &BlockMetrics, c: Checkpoint) -> usize {
        m.inner
            .as_ref()
            .expect("There are some metrics")
            .lock()
            .starts
            .get(&c)
            .unwrap()
            .len()
    }

    fn check_reporting_with_memory_excess(metrics: &BlockMetrics, checkpoint: Checkpoint) {
        for i in 1..(MAX_BLOCKS_PER_CHECKPOINT + 10) {
            metrics.report_block(BlockHash::random(), Instant::now(), checkpoint);
            assert_eq!(
                min(i, MAX_BLOCKS_PER_CHECKPOINT),
                starts_for(metrics, checkpoint)
            )
        }
    }

    #[test]
    fn registration_with_no_register_creates_empty_metrics() {
        let m = BlockMetrics::noop();
        m.report_block(BlockHash::random(), Instant::now(), Checkpoint::Ordered);
        assert!(m.inner.is_none());
    }

    #[test]
    fn should_keep_entries_up_to_defined_limit() {
        let m = register_dummy_metrics();
        check_reporting_with_memory_excess(&m, Checkpoint::Ordered);
    }

    #[test]
    fn should_manage_space_for_checkpoints_independently() {
        let m = register_dummy_metrics();
        check_reporting_with_memory_excess(&m, Checkpoint::Ordered);
        check_reporting_with_memory_excess(&m, Checkpoint::Imported);
    }

    #[test]
    fn given_not_monotonic_clock_when_report_block_is_called_repeatedly_code_does_not_panic() {
        let metrics = register_dummy_metrics();
        let earlier_timestamp = Instant::now();
        let later_timestamp = earlier_timestamp + Duration::new(0, 5);
        let hash = BlockHash::random();
        metrics.report_block(hash, later_timestamp, Checkpoint::Ordering);
        metrics.report_block(hash, earlier_timestamp, Checkpoint::Ordered);
    }
}
