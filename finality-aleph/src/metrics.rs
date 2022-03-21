use std::{
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    time::{Duration, Instant},
};

use log::{trace, warn};
use lru::LruCache;
use parking_lot::Mutex;
use prometheus_endpoint::{register, Gauge, PrometheusError, Registry, U64};
use sc_service::Arc;

// How many entries (block hash + timestamp) we keep in memory per one checkpoint type.
// Each entry takes 32B (Hash) + 16B (Instant), so a limit of 5000 gives ~234kB (per checkpoint).
// Notice that some issues like finalization stall may lead to incomplete metrics
// (e.g. when the gap between checkpoints for a block grows over `MAX_BLOCKS_PER_CHECKPOINT`).
const MAX_BLOCKS_PER_CHECKPOINT: usize = 5000;

pub trait Key: Hash + Eq + Debug + Copy {}
impl<T: Hash + Eq + Debug + Copy> Key for T {}

struct Inner<H: Key> {
    prev: HashMap<Checkpoint, Checkpoint>,
    gauges: HashMap<Checkpoint, Gauge<U64>>,
    starts: HashMap<Checkpoint, LruCache<H, Instant>>,
}

impl<H: Key> Inner<H> {
    fn report_block(&mut self, hash: H, checkpoint_time: Instant, checkpoint_type: Checkpoint) {
        trace!(target: "aleph-metrics", "Reporting block stage: {:?} (hash: {:?}, at: {:?}", checkpoint_type, hash, checkpoint_time);

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
                        warn!(target: "aleph-metrics", "Earlier metrics time {:?} is later that current one \
                        {:?}. Checkpoint type {:?}, block: {:?}",
                            *start, checkpoint_time, checkpoint_type, hash);
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
pub(crate) enum Checkpoint {
    Importing,
    Imported,
    Ordering,
    Ordered,
    Aggregating,
    Finalized,
}

#[derive(Clone)]
pub struct Metrics<H: Key> {
    inner: Arc<Mutex<Inner<H>>>,
}

impl<H: Key> Metrics<H> {
    pub fn register(registry: &Registry) -> Result<Self, PrometheusError> {
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
                register(Gauge::new(format!("aleph_{:?}", key), "no help")?, registry)?,
            );
        }

        let inner = Arc::new(Mutex::new(Inner {
            prev,
            gauges,
            starts: keys
                .iter()
                .map(|k| (*k, LruCache::new(MAX_BLOCKS_PER_CHECKPOINT)))
                .collect(),
        }));

        Ok(Self { inner })
    }

    pub(crate) fn report_block(
        &self,
        hash: H,
        checkpoint_time: Instant,
        checkpoint_type: Checkpoint,
    ) {
        self.inner
            .lock()
            .report_block(hash, checkpoint_time, checkpoint_type);
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::min;

    use super::*;

    fn starts_for<H: Key>(m: &Metrics<H>, c: Checkpoint) -> usize {
        m.inner.lock().starts.get(&c).unwrap().len()
    }

    fn check_reporting_with_memory_excess(metrics: &Metrics<usize>, checkpoint: Checkpoint) {
        for i in 1..(MAX_BLOCKS_PER_CHECKPOINT + 10) {
            metrics.report_block(i, Instant::now(), checkpoint);
            assert_eq!(
                min(i, MAX_BLOCKS_PER_CHECKPOINT),
                starts_for(metrics, checkpoint)
            )
        }
    }

    #[test]
    fn should_keep_entries_up_to_defined_limit() {
        let m = Metrics::<usize>::register(&Registry::new()).unwrap();
        check_reporting_with_memory_excess(&m, Checkpoint::Ordered);
    }

    #[test]
    fn should_manage_space_for_checkpoints_independently() {
        let m = Metrics::<usize>::register(&Registry::new()).unwrap();
        check_reporting_with_memory_excess(&m, Checkpoint::Ordered);
        check_reporting_with_memory_excess(&m, Checkpoint::Imported);
    }

    #[test]
    fn given_not_monotonic_clock_when_report_block_is_called_repeatedly_code_does_not_panic() {
        let metrics = Metrics::<usize>::register(&Registry::new()).unwrap();
        let earlier_timestamp = Instant::now();
        let later_timestamp = earlier_timestamp + Duration::new(0, 5);
        metrics.report_block(0, later_timestamp, Checkpoint::Ordering);
        metrics.report_block(0, earlier_timestamp, Checkpoint::Ordered);
    }
}
