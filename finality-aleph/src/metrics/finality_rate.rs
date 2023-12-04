use std::num::NonZeroUsize;

use log::warn;
use lru::LruCache;
use parking_lot::Mutex;
use primitives::{BlockHash, BlockNumber};
use sc_service::Arc;
use sp_core::{bounded_vec::BoundedVec, ConstU32};
use substrate_prometheus_endpoint::{register, Counter, PrometheusError, Registry, U64};

use super::Checkpoint;
use crate::metrics::LOG_TARGET;

const MAX_CACHE_SIZE: usize = 1800;
const MAX_INNER_SIZE: u32 = 64;

type ImportedHashesCache =
    Arc<Mutex<LruCache<BlockNumber, BoundedVec<BlockHash, ConstU32<MAX_INNER_SIZE>>>>>;

#[derive(Clone)]
pub enum FinalityRateMetrics {
    Prometheus {
        own_finalized: Counter<U64>,
        own_hopeless: Counter<U64>,
        imported_cache: ImportedHashesCache,
    },
    Noop,
}

impl FinalityRateMetrics {
    pub fn new(registry: Option<&Registry>) -> Result<Self, PrometheusError> {
        let registry = match registry {
            None => return Ok(FinalityRateMetrics::Noop),
            Some(registry) => registry,
        };

        Ok(FinalityRateMetrics::Prometheus {
            own_finalized: register(
                Counter::new("aleph_own_finalized_blocks", "no help")?,
                registry,
            )?,
            own_hopeless: register(
                Counter::new("aleph_own_hopeless_blocks", "no help")?,
                registry,
            )?,
            imported_cache: Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::new(MAX_CACHE_SIZE).unwrap(),
            ))),
        })
    }

    pub fn report_block(
        &self,
        block_hash: BlockHash,
        block_number: BlockNumber,
        checkpoint: Checkpoint,
        own: Option<bool>,
    ) {
        match checkpoint {
            Checkpoint::Imported => {
                if let Some(true) = own {
                    self.report_own_imported(block_hash, block_number);
                }
            }
            Checkpoint::Finalized => self.report_finalized(block_hash, block_number),
            _ => {}
        }
    }

    /// Stores the imported block's hash. Assumes that the imported block is own.
    fn report_own_imported(&self, hash: BlockHash, number: BlockNumber) {
        let mut imported_cache = match self {
            FinalityRateMetrics::Prometheus { imported_cache, .. } => imported_cache.lock(),
            FinalityRateMetrics::Noop => return,
        };

        let entry = imported_cache
            .get_or_insert_mut(number, BoundedVec::<_, ConstU32<MAX_INNER_SIZE>>::new);

        if entry.try_push(hash).is_err() {
            warn!(
                target: LOG_TARGET,
                "Finality Rate Metrics encountered too many own imported blocks at level {}",
                number
            );
        }
    }

    /// Counts the blocks at the level of `number` different than the passed block
    /// and reports them as hopeless. If `hash` is a hash of own block it will be found
    /// in `imported_cache` and reported as finalized.
    fn report_finalized(&self, hash: BlockHash, number: BlockNumber) {
        let (own_finalized, own_hopeless, imported_cache) = match self {
            FinalityRateMetrics::Prometheus {
                own_finalized,
                own_hopeless,
                imported_cache,
            } => (own_finalized, own_hopeless, imported_cache),
            FinalityRateMetrics::Noop => return,
        };

        let mut imported_cache = imported_cache.lock();
        if let Some(hashes) = imported_cache.get_mut(&number) {
            let new_hopeless_count = hashes.iter().filter(|h| **h != hash).count();
            own_hopeless.inc_by(new_hopeless_count as u64);
            own_finalized.inc_by((hashes.len() - new_hopeless_count) as u64);
        }
        imported_cache.pop(&number);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use primitives::{BlockHash, BlockNumber};
    use substrate_prometheus_endpoint::{Counter, Registry, U64};

    use crate::{metrics::finality_rate::ImportedHashesCache, FinalityRateMetrics};

    type FinalityRateMetricsInternals = (Counter<U64>, Counter<U64>, ImportedHashesCache);

    fn extract_internals(metrics: FinalityRateMetrics) -> FinalityRateMetricsInternals {
        match metrics {
            FinalityRateMetrics::Prometheus {
                own_finalized,
                own_hopeless,
                imported_cache,
            } => (own_finalized, own_hopeless, imported_cache),
            FinalityRateMetrics::Noop => panic!("metrics should have been initialized properly"),
        }
    }

    fn verify_state(
        metrics: &FinalityRateMetrics,
        expected_finalized: u64,
        expected_hopeless: u64,
        expected_cache: HashMap<BlockNumber, Vec<BlockHash>>,
    ) {
        let (finalized, hopeless, cache) = extract_internals(metrics.clone());
        let cache = cache.lock();
        assert_eq!(finalized.get(), expected_finalized);
        assert_eq!(hopeless.get(), expected_hopeless);

        // verify caches are equal
        assert_eq!(expected_cache.len(), cache.len());
        for (level, expected_hashes) in expected_cache {
            assert!(cache.contains(&level));
            let hashes = cache.peek(&level).unwrap();
            assert_eq!(expected_hashes, hashes.clone().into_inner());
        }
    }

    #[test]
    fn imported_cache_behaves_properly() {
        let metrics = FinalityRateMetrics::new(Some(&Registry::new())).unwrap();

        verify_state(&metrics, 0, 0, HashMap::new());

        let hash0 = BlockHash::random();
        metrics.report_own_imported(hash0, 0);

        verify_state(&metrics, 0, 0, HashMap::from([(0, vec![hash0])]));

        let hash1 = BlockHash::random();
        metrics.report_own_imported(hash1, 1);

        verify_state(
            &metrics,
            0,
            0,
            HashMap::from([(0, vec![hash0]), (1, vec![hash1])]),
        );

        let hash2 = BlockHash::random();
        metrics.report_own_imported(hash2, 1);

        verify_state(
            &metrics,
            0,
            0,
            HashMap::from([(0, vec![hash0]), (1, vec![hash1, hash2])]),
        );

        metrics.report_finalized(hash0, 0);

        verify_state(&metrics, 1, 0, HashMap::from([(1, vec![hash1, hash2])]));

        metrics.report_finalized(BlockHash::random(), 1);

        verify_state(&metrics, 1, 2, HashMap::new());
    }
}
