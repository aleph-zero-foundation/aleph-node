use std::{
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    time::{Duration, Instant},
};

use log::{trace, warn};
use lru::LruCache;
use parking_lot::Mutex;
use prometheus_endpoint::{
    exponential_buckets, prometheus::HistogramTimer, register, Counter, Gauge, Histogram,
    HistogramOpts, Opts, PrometheusError, Registry, U64,
};
use sc_service::Arc;

use crate::network::Protocol;

// How many entries (block hash + timestamp) we keep in memory per one checkpoint type.
// Each entry takes 32B (Hash) + 16B (Instant), so a limit of 5000 gives ~234kB (per checkpoint).
// Notice that some issues like finalization stall may lead to incomplete metrics
// (e.g. when the gap between checkpoints for a block grows over `MAX_BLOCKS_PER_CHECKPOINT`).
const MAX_BLOCKS_PER_CHECKPOINT: usize = 5000;

pub trait Key: Hash + Eq + Debug + Copy + Send + 'static {}
impl<T: Hash + Eq + Debug + Copy + Send + 'static> Key for T {}

const LOG_TARGET: &str = "aleph-metrics";

struct Inner<H: Key> {
    prev: HashMap<Checkpoint, Checkpoint>,
    gauges: HashMap<Checkpoint, Gauge<U64>>,
    starts: HashMap<Checkpoint, LruCache<H, Instant>>,
    sync_broadcast_counter: Counter<U64>,
    sync_send_request_for_counter: Counter<U64>,
    sync_send_to_counter: Counter<U64>,
    sync_handle_state_counter: Counter<U64>,
    sync_handle_justifications_counter: Counter<U64>,
    sync_handle_request_counter: Counter<U64>,
    sync_handle_task_counter: Counter<U64>,
    sync_handle_block_imported_counter: Counter<U64>,
    sync_handle_block_finalized_counter: Counter<U64>,

    network_send_times: HashMap<Protocol, Histogram>,
}

impl<H: Key> Inner<H> {
    fn report_block(&mut self, hash: H, checkpoint_time: Instant, checkpoint_type: Checkpoint) {
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

    fn start_sending_in(&self, protocol: Protocol) -> HistogramTimer {
        self.network_send_times[&protocol].start_timer()
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

fn protocol_name(protocol: Protocol) -> String {
    use Protocol::*;
    match protocol {
        Authentication => "authentication",
        BlockSync => "block_sync",
    }
    .to_string()
}

#[derive(Clone)]
pub struct Metrics<H: Key> {
    inner: Option<Arc<Mutex<Inner<H>>>>,
}

impl<H: Key> Metrics<H> {
    pub fn noop() -> Metrics<H> {
        Metrics { inner: None }
    }

    pub fn new(registry: &Registry) -> Result<Metrics<H>, PrometheusError> {
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

        use Protocol::*;
        let mut network_send_times = HashMap::new();
        for key in [Authentication, BlockSync] {
            network_send_times.insert(
                key,
                register(
                    Histogram::with_opts(HistogramOpts {
                        common_opts: Opts {
                            namespace: "gossip_network".to_string(),
                            subsystem: protocol_name(key),
                            name: "send_duration".to_string(),
                            help: "How long did it take for substrate to send a message."
                                .to_string(),
                            const_labels: Default::default(),
                            variable_labels: Default::default(),
                        },
                        buckets: exponential_buckets(0.001, 1.26, 30)?,
                    })?,
                    registry,
                )?,
            );
        }

        let inner = Some(Arc::new(Mutex::new(Inner {
            prev,
            gauges,
            starts: keys
                .iter()
                .map(|k| (*k, LruCache::new(MAX_BLOCKS_PER_CHECKPOINT)))
                .collect(),
            sync_broadcast_counter: register(
                Counter::new("aleph_sync_broadcast", "no help")?,
                registry,
            )?,
            sync_send_request_for_counter: register(
                Counter::new("aleph_sync_send_request_for", "no help")?,
                registry,
            )?,
            sync_send_to_counter: register(
                Counter::new("aleph_sync_send_to", "no help")?,
                registry,
            )?,
            sync_handle_state_counter: register(
                Counter::new("aleph_sync_handle_state", "no help")?,
                registry,
            )?,
            sync_handle_justifications_counter: register(
                Counter::new("aleph_sync_handle_justifications", "no help")?,
                registry,
            )?,
            sync_handle_request_counter: register(
                Counter::new("aleph_sync_handle_request", "no help")?,
                registry,
            )?,
            sync_handle_task_counter: register(
                Counter::new("aleph_sync_handle_task", "no help")?,
                registry,
            )?,
            sync_handle_block_imported_counter: register(
                Counter::new("aleph_sync_handle_block_imported", "no help")?,
                registry,
            )?,
            sync_handle_block_finalized_counter: register(
                Counter::new("aleph_sync_handle_block_finalized", "no help")?,
                registry,
            )?,
            network_send_times,
        })));

        Ok(Metrics { inner })
    }

    pub(crate) fn report_block(
        &self,
        hash: H,
        checkpoint_time: Instant,
        checkpoint_type: Checkpoint,
    ) {
        if let Some(inner) = &self.inner {
            inner
                .lock()
                .report_block(hash, checkpoint_time, checkpoint_type);
        }
    }

    pub fn report_sync_broadcast(&self) {
        if let Some(inner) = &self.inner {
            inner.lock().sync_broadcast_counter.inc();
        }
    }

    pub fn report_sync_send_request_for(&self) {
        if let Some(inner) = &self.inner {
            inner.lock().sync_send_request_for_counter.inc();
        }
    }

    pub fn report_sync_send_to(&self) {
        if let Some(inner) = &self.inner {
            inner.lock().sync_send_to_counter.inc();
        }
    }

    pub fn report_sync_handle_state(&self) {
        if let Some(inner) = &self.inner {
            inner.lock().sync_handle_state_counter.inc();
        }
    }

    pub fn report_sync_handle_justifications(&self) {
        if let Some(inner) = &self.inner {
            inner.lock().sync_handle_justifications_counter.inc();
        }
    }

    pub fn report_sync_handle_request(&self) {
        if let Some(inner) = &self.inner {
            inner.lock().sync_handle_request_counter.inc();
        }
    }

    pub fn report_sync_handle_task(&self) {
        if let Some(inner) = &self.inner {
            inner.lock().sync_handle_task_counter.inc();
        }
    }

    pub fn report_sync_handle_block_imported(&self) {
        if let Some(inner) = &self.inner {
            inner.lock().sync_handle_block_imported_counter.inc();
        }
    }

    pub fn report_sync_handle_block_finalized(&self) {
        if let Some(inner) = &self.inner {
            inner.lock().sync_handle_block_finalized_counter.inc();
        }
    }

    pub fn start_sending_in(&self, protocol: Protocol) -> Option<HistogramTimer> {
        self.inner
            .as_ref()
            .map(|inner| inner.lock().start_sending_in(protocol))
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::min;

    use super::*;

    fn register_dummy_metrics() -> Metrics<usize> {
        Metrics::<usize>::new(&Registry::new()).unwrap()
    }

    fn starts_for<H: Key>(m: &Metrics<H>, c: Checkpoint) -> usize {
        m.inner
            .as_ref()
            .expect("There are some metrics")
            .lock()
            .starts
            .get(&c)
            .unwrap()
            .len()
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
    fn registration_with_no_register_creates_empty_metrics() {
        let m = Metrics::<usize>::noop();
        m.report_block(0, Instant::now(), Checkpoint::Ordered);
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
        metrics.report_block(0, later_timestamp, Checkpoint::Ordering);
        metrics.report_block(0, earlier_timestamp, Checkpoint::Ordered);
    }
}
