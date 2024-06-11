use std::{
    num::NonZeroUsize,
    sync::Arc,
    time::{Duration, Instant},
};

use lru::LruCache;
use parking_lot::Mutex;
use substrate_prometheus_endpoint::{
    register, Counter, Histogram, HistogramOpts, PrometheusError, Registry, U64,
};

use crate::metrics::{exponential_buckets_two_sided, timing::Clock};

// Size of transaction cache: 32B (Hash) + 16B (Instant) * `100_000` is approximately 4.8MB
const TRANSACTION_CACHE_SIZE: usize = 100_000;
const BUCKETS_FACTOR: f64 = 1.4;

#[derive(Clone)]
pub enum TransactionPoolMetrics<TxHash, C> {
    Prometheus {
        time_till_block_inclusion: Histogram,
        transactions_not_seen_in_the_pool: Counter<U64>,
        cache: Arc<Mutex<LruCache<TxHash, Instant>>>,
        clock: C,
    },
    Noop,
}

impl<TxHash: std::hash::Hash + Eq, C: Clock> TransactionPoolMetrics<TxHash, C> {
    pub fn new(registry: Option<&Registry>, clock: C) -> Result<Self, PrometheusError> {
        let registry = match registry {
            None => return Ok(Self::Noop),
            Some(registry) => registry,
        };

        Ok(Self::Prometheus {
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
                registry,
            )?,
            transactions_not_seen_in_the_pool: register(
                Counter::new(
                    "aleph_transactions_not_seen_in_the_pool",
                    "\
                Number of transactions that were reported to be in block before reporting of \
                being in the ready queue in the transaction pool. This could happen \
                for many reasons, e.g. when a transaction has been added to the future pool, \
                has been submitted locally, or because of a race condition \
                (especially probable when there is an increased transaction load)",
                )?,
                registry,
            )?,
            cache: Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::new(TRANSACTION_CACHE_SIZE)
                    .expect("the cache size is a non-zero constant"),
            ))),
            clock,
        })
    }

    pub fn report_in_pool(&self, hash: TxHash) {
        if let Self::Prometheus { cache, clock, .. } = self {
            // Putting new transaction can evict the oldest one. However, even if the
            // removed transaction was actually still in the pool, we don't have
            // any guarantees that it would be eventually included in the block.
            // Therefore, we ignore such transaction.
            let cache = &mut *cache.lock();
            cache.put(hash, clock.now());
        }
    }

    pub fn report_in_block(&self, hash: TxHash) {
        if let Self::Prometheus {
            time_till_block_inclusion,
            transactions_not_seen_in_the_pool,
            cache,
            ..
        } = self
        {
            let cache = &mut *cache.lock();
            let elapsed = match cache.pop(&hash) {
                Some(insert_time) => insert_time.elapsed(),
                None => {
                    // Either it was never in the pool (e.g. submitted locally), or we've got BlockImport
                    // notification faster than transaction in pool one. The latter is much more likely,
                    // so we report it as zero.
                    transactions_not_seen_in_the_pool.inc();
                    Duration::ZERO
                }
            };
            time_till_block_inclusion.observe(elapsed.as_secs_f64() * 1000.);
        }
    }
}

#[cfg(test)]
pub mod test {
    use std::{
        collections::HashMap,
        hash::Hash,
        sync::Arc,
        time::{Duration, Instant},
    };

    use futures::{future, FutureExt, Stream, StreamExt};
    use parity_scale_codec::Encode;
    use sc_basic_authorship::ProposerFactory;
    use sc_block_builder::BlockBuilderBuilder;
    use sc_client_api::{
        BlockBackend, BlockImportNotification, BlockchainEvents, FinalityNotification,
        HeaderBackend,
    };
    use sc_transaction_pool::{BasicPool, FullChainApi};
    use sc_transaction_pool_api::{
        ImportNotificationStream, MaintainedTransactionPool, TransactionPool,
    };
    use sp_consensus::{BlockOrigin, DisableProofRecording, Environment, Proposer as _};
    use sp_runtime::{traits::Block as BlockT, transaction_validity::TransactionSource};
    use substrate_prometheus_endpoint::{Histogram, Registry};
    use substrate_test_client::TestClientBuilder;
    use substrate_test_runtime::{Extrinsic, ExtrinsicBuilder, Transfer};
    use substrate_test_runtime_client::{AccountKeyring, ClientBlockImportExt, ClientExt};

    use crate::{
        metrics::{
            slo::{Hashing, TxHash},
            timing::DefaultClock,
            transaction_pool::TransactionPoolMetrics,
        },
        testing::mocks::{TBlock, THash, TestClient, TestClientBuilderExt},
    };

    type TChainApi = FullChainApi<TestClient, TBlock>;
    type FullTransactionPool = BasicPool<TChainApi, TBlock>;
    type TProposerFactory = ProposerFactory<FullTransactionPool, TestClient, DisableProofRecording>;

    pub struct TestTransactionPoolSetup {
        pub client: Arc<TestClient>,
        pub pool: Arc<FullTransactionPool>,
        pub proposer_factory: TProposerFactory,
    }

    impl TestTransactionPoolSetup {
        pub fn new(client: Arc<TestClient>) -> Self {
            let spawner = sp_core::testing::TaskExecutor::new();
            let pool = BasicPool::new_full(
                Default::default(),
                true.into(),
                None,
                spawner.clone(),
                client.clone(),
            );

            let proposer_factory =
                ProposerFactory::new(spawner, client.clone(), pool.clone(), None, None);

            TestTransactionPoolSetup {
                client,
                pool,
                proposer_factory,
            }
        }

        pub fn import_notification_stream(&self) -> ImportNotificationStream<TxHash> {
            self.pool.import_notification_stream()
        }

        pub async fn propose_block(&mut self, at: THash, weight_limit: Option<usize>) -> TBlock {
            let proposer = self
                .proposer_factory
                .init(&self.client.expect_header(at).unwrap())
                .await
                .unwrap();

            let block = proposer
                .propose(
                    Default::default(),
                    Default::default(),
                    Duration::from_secs(30),
                    weight_limit,
                )
                .await
                .unwrap()
                .block;

            self.import_block(block).await
        }

        pub async fn import_block(&mut self, block: TBlock) -> TBlock {
            let stream = self.client.every_import_notification_stream();
            self.client
                .import(BlockOrigin::Own, block.clone())
                .await
                .unwrap();

            let notification = stream
                .filter(|notification| future::ready(notification.hash == block.hash()))
                .next()
                .await
                .expect("Notification was sent");

            if notification.is_new_best {
                self.pool.maintain(notification.try_into().unwrap()).await;
            }

            block
        }

        pub async fn finalize(&mut self, hash: THash) {
            let stream = self.client.finality_notification_stream();
            self.client.finalize_block(hash, None).unwrap();
            let notification = stream
                .filter(|notification| future::ready(notification.hash == hash))
                .next()
                .await
                .expect("Notification was sent");

            self.pool.maintain(notification.into()).await;
        }

        pub fn extrinsic(
            &self,
            sender: AccountKeyring,
            receiver: AccountKeyring,
            nonce: u64,
        ) -> Extrinsic {
            let tx = Transfer {
                amount: Default::default(),
                nonce,
                from: sender.into(),
                to: receiver.into(),
            };
            ExtrinsicBuilder::new_transfer(tx).build()
        }

        pub async fn submit(&mut self, at: &THash, xt: Extrinsic) {
            self.pool
                .submit_one(*at, TransactionSource::External, xt.into())
                .await
                .unwrap();
        }
    }

    // Transaction pool metrics tests
    struct TestSetup {
        pub pool: TestTransactionPoolSetup,
        pub metrics: TransactionPoolMetrics<THash, DefaultClock>,
        pub block_import_notifications:
            Box<dyn Stream<Item = BlockImportNotification<TBlock>> + Unpin>,
        pub finality_notifications: Box<dyn Stream<Item = FinalityNotification<TBlock>> + Unpin>,
        pub pool_import_notifications: ImportNotificationStream<TxHash>,
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
            let pool_import_notifications = pool.import_notification_stream();

            let registry = Registry::new();
            let metrics =
                TransactionPoolMetrics::new(Some(&registry), DefaultClock).expect("metrics");

            TestSetup {
                pool,
                metrics,
                block_import_notifications,
                finality_notifications,
                pool_import_notifications,
            }
        }

        fn genesis(&self) -> THash {
            self.pool.client.info().genesis_hash
        }

        fn transactions_histogram(&self) -> &Histogram {
            match &self.metrics {
                TransactionPoolMetrics::Prometheus {
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
                let body = self
                    .pool
                    .client
                    .block_body(block.expect("stream should not end").hash)
                    .expect("block should exist")
                    .expect("block should have body");
                for xt in body {
                    let hash = xt.using_encoded(<Hashing as sp_runtime::traits::Hash>::hash);
                    self.metrics.report_in_block(hash);
                }
                block_imported_notifications += 1;
            }
            while self.finality_notifications.next().now_or_never().is_some() {
                finality_notifications += 1;
            }
            while let Some(transaction) = self.pool_import_notifications.next().now_or_never() {
                self.metrics
                    .report_in_pool(transaction.expect("stream should not end"));
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
        let mut block_1b_builder = BlockBuilderBuilder::new(&*setup.pool.client)
            .on_parent_block(genesis)
            .with_parent_block_number(0)
            .build()
            .unwrap();

        block_1b_builder.push(xt1.into()).unwrap();
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
            .extrinsic(AccountKeyring::Charlie, AccountKeyring::Eve, 0);

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
