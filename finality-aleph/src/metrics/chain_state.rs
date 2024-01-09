use std::{
    fmt::Display,
    num::NonZeroUsize,
    time::{Duration, Instant},
};

use futures::{stream::FusedStream, StreamExt};
use lru::LruCache;
use sc_client_api::{
    BlockBackend, BlockImportNotification, FinalityNotification, FinalityNotifications,
    ImportNotifications,
};
use sp_api::{BlockT, HeaderT};
use sp_blockchain::{lowest_common_ancestor, HeaderMetadata};
use sp_runtime::{
    traits::{Extrinsic, Zero},
    Saturating,
};
use substrate_prometheus_endpoint::{
    register, Counter, Gauge, Histogram, HistogramOpts, PrometheusError, Registry, U64,
};

use crate::{
    metrics::{exponential_buckets_two_sided, TransactionPoolInfoProvider},
    BlockNumber,
};

// Size of transaction cache: 32B (Hash) + 16B (Instant) * `100_000` is approximately 4.8MB
const TRANSACTION_CACHE_SIZE: usize = 100_000;

const BUCKETS_FACTOR: f64 = 1.4;

#[derive(Debug)]
pub enum Error {
    NoRegistry,
    UnableToCreateMetrics(PrometheusError),
    BlockImportStreamClosed,
    FinalizedBlocksStreamClosed,
    TransactionStreamClosed,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoRegistry => write!(f, "Registry can not be empty."),
            Error::UnableToCreateMetrics(e) => {
                write!(f, "Failed to create metrics: {e}.")
            }
            Error::BlockImportStreamClosed => {
                write!(f, "Block import notification stream ended unexpectedly.")
            }
            Error::FinalizedBlocksStreamClosed => {
                write!(f, "Finality notification stream ended unexpectedly.")
            }
            Error::TransactionStreamClosed => {
                write!(f, "Transaction stream ended unexpectedly.")
            }
        }
    }
}

enum ChainStateMetrics {
    Prometheus {
        top_finalized_block: Gauge<U64>,
        best_block: Gauge<U64>,
        reorgs: Histogram,
        time_till_block_inclusion: Histogram,
        transactions_not_seen_in_the_pool: Counter<U64>,
    },
    Noop,
}

impl ChainStateMetrics {
    fn new(registry: Option<Registry>) -> Result<Self, PrometheusError> {
        let registry = match registry {
            Some(registry) => registry,
            None => return Ok(ChainStateMetrics::Noop),
        };

        Ok(ChainStateMetrics::Prometheus {
            top_finalized_block: register(
                Gauge::new("aleph_top_finalized_block", "no help")?,
                &registry,
            )?,
            best_block: register(Gauge::new("aleph_best_block", "no help")?, &registry)?,
            reorgs: register(
                Histogram::with_opts(
                    HistogramOpts::new("aleph_reorgs", "Number of reorgs by length")
                        .buckets(vec![1., 2., 3., 5., 10.]),
                )?,
                &registry,
            )?,
            time_till_block_inclusion: register(
                Histogram::with_opts(
                    HistogramOpts::new(
                        "aleph_transaction_to_block_time",
                        "Time from becoming ready in the pool to inclusion in some valid block.",
                    )
                    .buckets(exponential_buckets_two_sided(
                        2000.0,
                        BUCKETS_FACTOR,
                        2,
                        8,
                    )?),
                )?,
                &registry,
            )?,
            transactions_not_seen_in_the_pool: register(
                Counter::new("aleph_transactions_not_seen_in_the_pool", "no help")?,
                &registry,
            )?,
        })
    }

    fn update_best_block(&self, number: BlockNumber) {
        if let ChainStateMetrics::Prometheus { best_block, .. } = self {
            best_block.set(number as u64)
        }
    }

    fn update_top_finalized_block(&self, number: BlockNumber) {
        if let ChainStateMetrics::Prometheus {
            top_finalized_block,
            ..
        } = self
        {
            top_finalized_block.set(number as u64);
        }
    }

    fn report_reorg(&self, length: BlockNumber) {
        if let ChainStateMetrics::Prometheus { reorgs, .. } = self {
            reorgs.observe(length as f64);
        }
    }

    fn report_transaction_in_block(&self, elapsed: Duration) {
        if let ChainStateMetrics::Prometheus {
            time_till_block_inclusion,
            ..
        } = self
        {
            time_till_block_inclusion.observe(elapsed.as_secs_f64() * 1000.);
        }
    }

    fn report_transaction_not_seen_in_the_pool(&self) {
        if let ChainStateMetrics::Prometheus {
            transactions_not_seen_in_the_pool,
            ..
        } = self
        {
            transactions_not_seen_in_the_pool.inc();
        }
    }
}

pub async fn run_chain_state_metrics<
    X,
    HE: HeaderT<Number = BlockNumber, Hash = B::Hash>,
    B: BlockT<Header = HE, Extrinsic = X>,
    BE: HeaderMetadata<B> + BlockBackend<B>,
    TP: TransactionPoolInfoProvider<Extrinsic = X>,
>(
    backend: &BE,
    mut import_notifications: ImportNotifications<B>,
    mut finality_notifications: FinalityNotifications<B>,
    registry: Option<Registry>,
    mut transaction_pool_info_provider: TP,
) -> Result<(), Error> {
    if registry.is_none() {
        return Err(Error::NoRegistry);
    }
    if import_notifications.is_terminated() {
        return Err(Error::BlockImportStreamClosed);
    }
    if finality_notifications.is_terminated() {
        return Err(Error::FinalizedBlocksStreamClosed);
    }

    let metrics = ChainStateMetrics::new(registry).map_err(Error::UnableToCreateMetrics)?;
    let mut cache: LruCache<TP::TxHash, Instant> = LruCache::new(
        NonZeroUsize::new(TRANSACTION_CACHE_SIZE).expect("the cache size is a non-zero constant"),
    );

    let mut previous_best: Option<HE> = None;

    loop {
        tokio::select! {
            maybe_block = import_notifications.next() => {
                let block = maybe_block.ok_or(Error::BlockImportStreamClosed)?;
                handle_block_imported(
                    block,
                    backend,
                    &metrics,
                    &mut transaction_pool_info_provider,
                    &mut cache,
                    &mut previous_best,
                );
            },
            maybe_block = finality_notifications.next() => {
                let block = maybe_block.ok_or(Error::FinalizedBlocksStreamClosed)?;
                handle_block_finalized(block, &metrics);
            },
            maybe_transaction = transaction_pool_info_provider.next_transaction() => {
                let hash = maybe_transaction.ok_or(Error::TransactionStreamClosed)?;
                handle_transaction_in_pool(hash, &mut cache);
            }
        }
    }
}

fn handle_block_imported<
    X,
    HE: HeaderT<Number = BlockNumber, Hash = B::Hash>,
    B: BlockT<Header = HE, Extrinsic = X>,
    BE: HeaderMetadata<B> + BlockBackend<B>,
    TP: TransactionPoolInfoProvider<Extrinsic = X>,
>(
    block: BlockImportNotification<B>,
    backend: &BE,
    metrics: &ChainStateMetrics,
    transaction_pool_info_provider: &mut TP,
    cache: &mut LruCache<TP::TxHash, Instant>,
    previous_best: &mut Option<HE>,
) {
    if block.is_new_best {
        metrics.update_best_block(*block.header.number());
        if let Some(reorg_len) = detect_reorgs(backend, previous_best.clone(), block.header.clone())
        {
            metrics.report_reorg(reorg_len);
        }
        *previous_best = Some(block.header);
    }
    if let Ok(Some(body)) = backend.block_body(block.hash) {
        report_transactions_included_in_block(
            transaction_pool_info_provider,
            &body,
            metrics,
            cache,
        );
    }
}

fn handle_block_finalized<HE: HeaderT<Number = BlockNumber>, B: BlockT<Header = HE>>(
    block: FinalityNotification<B>,
    metrics: &ChainStateMetrics,
) {
    // Sometimes finalization can also cause best block update. However,
    // RPC best block subscription won't notify about that immediately, so
    // we also don't update there. Also in that case, substrate sets best_block to
    // the newly finalized block (see test), so the best block will be updated
    // after importing anything on the newly finalized branch.
    metrics.update_top_finalized_block(*block.header.number());
}

fn handle_transaction_in_pool<TxHash: std::hash::Hash + Eq>(
    hash: TxHash,
    cache: &mut LruCache<TxHash, Instant>,
) {
    // Putting new transaction can evict the oldest one. However, even if the
    // removed transaction was actually still in the pool, we don't have
    // any guarantees that it would be eventually included in the block.
    // Therefore, we ignore such transaction.
    cache.put(hash, Instant::now());
}

fn detect_reorgs<HE: HeaderT<Hash = B::Hash>, B: BlockT<Header = HE>, BE: HeaderMetadata<B>>(
    backend: &BE,
    prev_best: Option<HE>,
    best: HE,
) -> Option<HE::Number> {
    let prev_best = prev_best?;
    if best.hash() == prev_best.hash() || *best.parent_hash() == prev_best.hash() {
        // Quit early when no change or the best is a child of the previous best.
        return None;
    }
    let lca = lowest_common_ancestor(backend, best.hash(), prev_best.hash()).ok()?;
    let len = prev_best.number().saturating_sub(lca.number);
    if len == HE::Number::zero() {
        return None;
    }
    Some(len)
}

fn report_transactions_included_in_block<
    'a,
    TP: TransactionPoolInfoProvider,
    I: IntoIterator<Item = &'a TP::Extrinsic>,
>(
    pool: &'a TP,
    body: I,
    metrics: &ChainStateMetrics,
    cache: &mut LruCache<TP::TxHash, Instant>,
) where
    <TP as TransactionPoolInfoProvider>::TxHash: std::hash::Hash + PartialEq + Eq,
{
    for xt in body {
        let hash = pool.hash_of(xt);
        if let Some(insert_time) = cache.pop(&hash) {
            let elapsed = insert_time.elapsed();
            metrics.report_transaction_in_block(elapsed);
        } else if let Some(true) = xt.is_signed() {
            // Either it was never in the pool (eg. submitted locally), or we've got BlockImport
            // notification faster than transaction in pool one. The latter is more likely,
            // so we report it as zero.
            metrics.report_transaction_in_block(Duration::ZERO);
            metrics.report_transaction_not_seen_in_the_pool();
        }
    }
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, sync::Arc};

    use futures::{FutureExt, Stream};
    use parity_scale_codec::Encode;
    use sc_block_builder::BlockBuilderProvider;
    use sc_client_api::{BlockchainEvents, HeaderBackend};
    use substrate_test_runtime_client::AccountKeyring;

    use super::*;
    use crate::{
        metrics::transaction_pool::test::TestTransactionPoolSetup,
        testing::{
            client_chain_builder::ClientChainBuilder,
            mocks::{TBlock, THash, TestClientBuilder, TestClientBuilderExt},
        },
    };

    #[tokio::test]
    async fn when_finalizing_with_reorg_best_block_is_set_to_that_finalized_block() {
        let client = Arc::new(TestClientBuilder::new().build());
        let client_builder = Arc::new(TestClientBuilder::new().build());
        let mut chain_builder = ClientChainBuilder::new(client.clone(), client_builder);

        let chain_a = chain_builder
            .build_and_import_branch_above(&chain_builder.genesis_hash(), 5)
            .await;

        // (G) - A1 - A2 - A3 - A4 - A5;

        assert_eq!(
            client.chain_info().finalized_hash,
            chain_builder.genesis_hash()
        );
        assert_eq!(client.chain_info().best_number, 5);

        let chain_b = chain_builder
            .build_and_import_branch_above(&chain_a[0].header.hash(), 3)
            .await;
        chain_builder.finalize_block(&chain_b[0].header.hash());

        // (G) - (A1) - A2 - A3 - A4 - A5
        //         \
        //          (B2) - B3 - B4

        assert_eq!(client.chain_info().best_number, 2);
        assert_eq!(client.chain_info().finalized_hash, chain_b[0].header.hash());
    }

    #[tokio::test]
    async fn test_reorg_detection() {
        let client = Arc::new(TestClientBuilder::new().build());
        let client_builder = Arc::new(TestClientBuilder::new().build());
        let mut chain_builder = ClientChainBuilder::new(client.clone(), client_builder);

        let a = chain_builder
            .build_and_import_branch_above(&chain_builder.genesis_hash(), 5)
            .await;
        let b = chain_builder
            .build_and_import_branch_above(&a[0].header.hash(), 3)
            .await;
        let c = chain_builder
            .build_and_import_branch_above(&a[2].header.hash(), 2)
            .await;

        //                  - C0 - C1
        //                /
        // G - A0 - A1 - A2 - A3 - A4
        //      \
        //        - B0 - B1 - B2

        for (prev, new_best, expected) in [
            (&a[1], &a[2], None),
            (&a[1], &a[4], None),
            (&a[1], &a[1], None),
            (&a[2], &b[0], Some(2)),
            (&b[0], &a[2], Some(1)),
            (&c[1], &b[2], Some(4)),
        ] {
            assert_eq!(
                detect_reorgs(
                    client.as_ref(),
                    Some(prev.header().clone()),
                    new_best.header().clone()
                ),
                expected,
            );
        }
    }

    // Transaction pool metrics tests
    struct TestSetup {
        pub pool: TestTransactionPoolSetup,
        pub metrics: ChainStateMetrics,
        pub cache: LruCache<THash, Instant>,
        pub block_import_notifications:
            Box<dyn Stream<Item = BlockImportNotification<TBlock>> + Unpin>,
        pub finality_notifications: Box<dyn Stream<Item = FinalityNotification<TBlock>> + Unpin>,
    }

    #[derive(PartialEq, Eq, Hash, Debug)]
    enum NotificationType {
        BlockImport,
        Finality,
        Transaction,
    }

    impl TestSetup {
        fn new() -> Self {
            let client = Arc::new(TestClientBuilder::new().build());

            let block_import_notifications =
                Box::new(client.every_import_notification_stream().fuse());
            let finality_notifications = Box::new(client.finality_notification_stream().fuse());

            let pool = TestTransactionPoolSetup::new(client);

            let registry = Registry::new();
            let metrics = ChainStateMetrics::new(Some(registry)).expect("metrics");
            let cache = LruCache::new(NonZeroUsize::new(10).expect("cache"));

            TestSetup {
                pool,
                metrics,
                cache,
                block_import_notifications,
                finality_notifications,
            }
        }

        fn genesis(&self) -> THash {
            self.pool.client.info().genesis_hash
        }

        fn transactions_histogram(&self) -> &Histogram {
            match &self.metrics {
                ChainStateMetrics::Prometheus {
                    time_till_block_inclusion,
                    ..
                } => time_till_block_inclusion,
                _ => panic!("metrics"),
            }
        }

        fn process_notifications(&mut self) -> HashMap<NotificationType, usize> {
            let mut block_imported_notifications = 0;
            let mut finality_notifications = 0;
            let mut transaction_notifications = 0;

            while let Some(block) = self.block_import_notifications.next().now_or_never() {
                handle_block_imported(
                    block.expect("stream should not end"),
                    self.pool.client.as_ref(),
                    &self.metrics,
                    &mut self.pool.transaction_pool_info_provider,
                    &mut self.cache,
                    &mut None,
                );
                block_imported_notifications += 1;
            }
            while let Some(finality) = self.finality_notifications.next().now_or_never() {
                handle_block_finalized(finality.expect("stream should not end"), &self.metrics);
                finality_notifications += 1;
            }
            while let Some(transaction) = self
                .pool
                .transaction_pool_info_provider
                .next_transaction()
                .now_or_never()
            {
                handle_transaction_in_pool(
                    transaction.expect("stream should not end"),
                    &mut self.cache,
                );
                transaction_notifications += 1;
            }
            HashMap::from_iter(vec![
                (NotificationType::BlockImport, block_imported_notifications),
                (NotificationType::Finality, finality_notifications),
                (NotificationType::Transaction, transaction_notifications),
            ])
        }
    }

    fn blocks_imported(n: usize) -> HashMap<NotificationType, usize> {
        HashMap::from_iter(vec![
            (NotificationType::BlockImport, n),
            (NotificationType::Finality, 0),
            (NotificationType::Transaction, 0),
        ])
    }
    fn transactions(n: usize) -> HashMap<NotificationType, usize> {
        HashMap::from_iter(vec![
            (NotificationType::BlockImport, 0),
            (NotificationType::Finality, 0),
            (NotificationType::Transaction, n),
        ])
    }

    const EPS: Duration = Duration::from_nanos(1);

    #[tokio::test]
    async fn transactions_are_reported() {
        let mut setup = TestSetup::new();
        let genesis = setup.genesis();
        let xt = setup
            .pool
            .extrinsic(AccountKeyring::Alice, AccountKeyring::Bob, 0);

        let time_before_submit = Instant::now();
        setup.pool.submit(&genesis, xt).await;

        assert_eq!(
            setup.process_notifications(),
            transactions(1),
            "'In pool' notification wasn't sent"
        );
        let time_after_submit = Instant::now();

        tokio::time::sleep(Duration::from_millis(20)).await;

        let time_before_import = Instant::now();
        let _block_1 = setup.pool.propose_block(genesis, None).await;
        let pre_count = setup.transactions_histogram().get_sample_count();

        assert_eq!(
            setup.process_notifications(),
            blocks_imported(1),
            "Block import notification wasn't sent"
        );

        let time_after_import = Instant::now();

        let duration =
            Duration::from_secs_f64(setup.transactions_histogram().get_sample_sum() / 1000.);

        assert_eq!(pre_count, 0);
        assert_eq!(setup.transactions_histogram().get_sample_count(), 1);
        assert!(duration >= time_before_import - time_after_submit - EPS);
        assert!(duration <= time_after_import - time_before_submit + EPS);
    }

    #[tokio::test]
    async fn transactions_are_reported_only_if_ready_when_added_to_the_pool() {
        let mut setup = TestSetup::new();
        let genesis = setup.genesis();

        let xt1 = setup
            .pool
            .extrinsic(AccountKeyring::Alice, AccountKeyring::Bob, 0);
        let xt2 = setup
            .pool
            .extrinsic(AccountKeyring::Alice, AccountKeyring::Bob, 1);
        let xt3 = setup
            .pool
            .extrinsic(AccountKeyring::Alice, AccountKeyring::Bob, 2);

        setup.pool.submit(&genesis, xt2.clone()).await;

        // No notification for xt2 as it is not ready
        assert_eq!(
            setup.process_notifications(),
            transactions(0),
            "Future transactions should not be reported"
        );

        setup.pool.submit(&genesis, xt1.clone()).await;
        setup.pool.submit(&genesis, xt3.clone()).await;

        // Notifications for xt1 and xt3
        assert_eq!(setup.process_notifications(), transactions(2));

        let block_1 = setup.pool.propose_block(genesis, None).await;
        // Block import notification. xt1 notification never appears
        assert_eq!(setup.process_notifications(), blocks_imported(1));
        // All 3 extrinsics are included in the block
        assert_eq!(block_1.extrinsics.len(), 3);
    }

    #[tokio::test]
    async fn retracted_transactions_are_reported_only_once() {
        let mut setup = TestSetup::new();
        let genesis = setup.genesis();

        let xt1 = setup
            .pool
            .extrinsic(AccountKeyring::Alice, AccountKeyring::Bob, 0);
        let xt2 = setup
            .pool
            .extrinsic(AccountKeyring::Charlie, AccountKeyring::Dave, 0);

        setup.pool.submit(&genesis, xt1.clone()).await;
        setup.pool.submit(&genesis, xt2.clone()).await;

        // make sure import notifications are received before block import
        assert_eq!(setup.process_notifications(), transactions(2));

        let block_1a = setup.pool.propose_block(genesis, None).await;
        assert_eq!(block_1a.extrinsics.len(), 2);
        assert_eq!(setup.process_notifications(), blocks_imported(1));
        assert_eq!(setup.transactions_histogram().get_sample_count(), 2);

        let sum_before = setup.transactions_histogram().get_sample_sum();

        // external fork block with xt1
        let mut block_1b_builder = setup
            .pool
            .client
            .new_block_at(genesis, Default::default(), false)
            .unwrap();
        block_1b_builder.push(xt1).unwrap();
        let block_1b = block_1b_builder.build().unwrap().block;
        setup.pool.import_block(block_1b.clone()).await;
        setup.pool.finalize(block_1b.hash()).await;

        let block_2b = setup.pool.propose_block(block_1b.hash(), None).await;

        assert_eq!(block_2b.extrinsics.len(), 1);
        assert_eq!(setup.transactions_histogram().get_sample_count(), 2);
        assert_eq!(setup.transactions_histogram().get_sample_sum(), sum_before);
    }

    #[tokio::test]
    async fn transactions_skipped_in_block_authorship_are_not_reported_at_that_time() {
        let mut setup = TestSetup::new();
        let genesis = setup.genesis();

        let xt1 = setup
            .pool
            .extrinsic(AccountKeyring::Alice, AccountKeyring::Bob, 0);
        let xt2 = setup
            .pool
            .extrinsic(AccountKeyring::Dave, AccountKeyring::Eve, 0);

        setup.pool.submit(&genesis, xt1.clone()).await;
        setup.pool.submit(&genesis, xt2.clone()).await;
        assert_eq!(setup.process_notifications(), transactions(2));

        let time_after_submit = Instant::now();

        let block_1 = setup
            .pool
            .propose_block(genesis, Some(2 * xt1.encoded_size() - 1))
            .await;

        assert_eq!(setup.process_notifications(), blocks_imported(1));
        assert_eq!(block_1.extrinsics.len(), 1);
        assert_eq!(setup.transactions_histogram().get_sample_count(), 1);
        let sample_1 = setup.transactions_histogram().get_sample_sum();

        tokio::time::sleep(Duration::from_millis(10)).await;

        let time_before_block_2 = Instant::now();
        let block_2 = setup
            .pool
            .propose_block(block_1.hash(), Some(2 * xt1.encoded_size() - 1))
            .await;

        assert_eq!(setup.process_notifications(), blocks_imported(1));
        assert_eq!(block_2.extrinsics.len(), 1);
        assert_eq!(setup.transactions_histogram().get_sample_count(), 2);

        let sample_2 = setup.transactions_histogram().get_sample_sum() - sample_1;

        let duration = Duration::from_secs_f64(sample_2 / 1000.0);

        assert!(duration >= time_before_block_2 - time_after_submit - EPS);
    }
}
