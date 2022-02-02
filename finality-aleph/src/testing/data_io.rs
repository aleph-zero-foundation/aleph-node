use crate::data_io::{AlephData, AlephDataFor, AlephNetworkMessage, DataStore, DataStoreConfig};
use crate::network::{DataNetwork, RequestBlocks, SimpleNetwork};
use futures::{
    channel::{
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
    StreamExt,
};
use sc_block_builder::BlockBuilderProvider;
use sp_api::BlockId;
use sp_api::NumberFor;
use sp_consensus::BlockOrigin;
use sp_runtime::traits::Block as BlockT;
use sp_runtime::Digest;
use std::{default::Default, future::Future, sync::Arc, time::Duration};
use substrate_test_runtime_client::{
    runtime::Block, Backend, ClientBlockImportExt, DefaultTestClientBuilderExt, TestClient,
    TestClientBuilder, TestClientBuilderExt,
};
use tokio::time::timeout;

#[derive(Clone)]
struct TestBlockRequester<B: BlockT> {
    blocks: UnboundedSender<AlephDataFor<B>>,
    justifications: UnboundedSender<AlephDataFor<B>>,
}

impl<B: BlockT> TestBlockRequester<B> {
    fn new() -> (
        Self,
        UnboundedReceiver<AlephDataFor<B>>,
        UnboundedReceiver<AlephDataFor<B>>,
    ) {
        let (blocks_tx, blocks_rx) = mpsc::unbounded();
        let (justifications_tx, justifications_rx) = mpsc::unbounded();
        (
            TestBlockRequester {
                blocks: blocks_tx,
                justifications: justifications_tx,
            },
            blocks_rx,
            justifications_rx,
        )
    }
}

impl<B: BlockT> RequestBlocks<B> for TestBlockRequester<B> {
    fn request_justification(&self, hash: &B::Hash, number: NumberFor<B>) {
        self.justifications
            .unbounded_send(AlephData {
                hash: *hash,
                number,
            })
            .unwrap();
    }

    fn request_stale_block(&self, hash: B::Hash, number: NumberFor<B>) {
        self.blocks
            .unbounded_send(AlephData { hash, number })
            .unwrap();
    }

    fn clear_justification_requests(&self) {
        panic!("`clear_justification_requests` not implemented!")
    }
}

type TestData = Vec<AlephDataFor<Block>>;

impl AlephNetworkMessage<Block> for TestData {
    fn included_blocks(&self) -> Vec<AlephDataFor<Block>> {
        self.clone()
    }
}

struct TestHandler {
    client: Arc<TestClient>,
    block_requests_rx: UnboundedReceiver<AlephDataFor<Block>>,
    network_tx: UnboundedSender<TestData>,
    exit_data_store_tx: oneshot::Sender<()>,
}

impl AlephDataFor<Block> {
    fn from(block: Block) -> AlephDataFor<Block> {
        AlephData::new(block.header.hash(), block.header.number)
    }
}

impl TestHandler {
    /// Import block in test client
    async fn import_block(&mut self, block: Block, finalize: bool) {
        if finalize {
            self.client.import_as_final(BlockOrigin::Own, block.clone())
        } else {
            self.client.import(BlockOrigin::Own, block.clone())
        }
        .await
        .unwrap();
    }

    /// Build block in test client
    fn build_block(&mut self) -> Block {
        self.client
            .new_block(Default::default())
            .unwrap()
            .build()
            .unwrap()
            .block
    }

    /// Build block `at` in test client
    fn build_block_at(&mut self, at: u64, contents: Vec<u8>) -> Block {
        let mut digest = Digest::default();
        digest.push(sp_runtime::generic::DigestItem::Other(contents));
        self.client
            .new_block_at(&BlockId::Number(at), digest, false)
            .unwrap()
            .build()
            .unwrap()
            .block
    }

    /// Build and import blocks in test client
    async fn build_and_import_blocks(&mut self, n: u32, finalize: bool) -> TestData {
        let mut blocks = vec![];
        for _ in 0..n {
            let block = self.build_block();
            self.import_block(block.clone(), finalize).await;
            blocks.push(AlephData::from(block));
        }
        blocks
    }

    /// Sends data to Data Store
    fn send_data(&self, data: TestData) {
        self.network_tx.unbounded_send(data).unwrap()
    }

    /// Exits Data Store
    fn exit(self) {
        self.exit_data_store_tx.send(()).unwrap();
    }

    /// Receive next block request from Data Store
    async fn next_block_request(&mut self) -> AlephDataFor<Block> {
        self.block_requests_rx.next().await.unwrap()
    }
}

fn prepare_data_store() -> (
    impl Future<Output = ()>,
    TestHandler,
    impl DataNetwork<TestData>,
) {
    let client = Arc::new(TestClientBuilder::new().build());

    let (block_requester, block_requests_rx, _justification_requests_rx) =
        TestBlockRequester::new();
    let (sender_tx, _sender_rx) = mpsc::unbounded();
    let (network_tx, network_rx) = mpsc::unbounded();
    let test_network = SimpleNetwork::new(network_rx, sender_tx);
    let data_store_config = DataStoreConfig {
        available_blocks_cache_capacity: 1000,
        message_id_boundary: 100_000,
        periodic_maintenance_interval: Duration::from_millis(30),
        request_block_after: Duration::from_millis(50),
    };

    let (mut data_store, network) = DataStore::<
        Block,
        TestClient,
        Backend,
        TestBlockRequester<Block>,
        TestData,
        UnboundedReceiver<TestData>,
    >::new(
        client.clone(),
        block_requester,
        data_store_config,
        test_network,
    );
    let (exit_data_store_tx, exit_data_store_rx) = oneshot::channel();

    (
        async move {
            data_store.run(exit_data_store_rx).await;
        },
        TestHandler {
            client,
            block_requests_rx,
            network_tx,
            exit_data_store_tx,
        },
        network,
    )
}

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

#[tokio::test]
async fn sends_messages_with_imported_blocks() {
    let (task_handle, mut test_handler, mut network) = prepare_data_store();
    let data_store_handle = tokio::spawn(task_handle);

    let blocks = test_handler.build_and_import_blocks(4, false).await;

    test_handler.send_data(blocks.clone());

    let message = timeout(DEFAULT_TIMEOUT, network.next())
        .await
        .ok()
        .flatten()
        .expect("Did not receive message from Data Store");
    assert_eq!(message.included_blocks(), blocks);
    test_handler.exit();
    data_store_handle.await.unwrap();
}

#[tokio::test]
async fn sends_messages_after_import() {
    let (task_handle, mut test_handler, mut network) = prepare_data_store();
    let data_store_handle = tokio::spawn(task_handle);

    let block = test_handler.build_block();
    let data = AlephData::from(block.clone());

    test_handler.send_data(vec![data]);

    test_handler.import_block(block, false).await;

    let message = timeout(DEFAULT_TIMEOUT, network.next())
        .await
        .ok()
        .flatten()
        .expect("Did not receive message from Data Store");
    assert_eq!(message.included_blocks(), vec![data]);

    test_handler.exit();
    data_store_handle.await.unwrap();
}

#[tokio::test]
async fn sends_messages_with_number_lower_than_finalized() {
    let (task_handle, mut test_handler, mut network) = prepare_data_store();
    let data_store_handle = tokio::spawn(task_handle);

    test_handler.build_and_import_blocks(4, true).await;

    let mut blocks = Vec::new();
    for i in 0u64..4 {
        blocks.push(AlephData::from(
            test_handler.build_block_at(i, i.to_le_bytes().to_vec()),
        ));
    }

    test_handler.send_data(blocks.clone());

    let message = timeout(DEFAULT_TIMEOUT, network.next())
        .await
        .ok()
        .flatten()
        .expect("Did not receive message from Data Store");
    assert_eq!(message.included_blocks(), blocks);

    test_handler.exit();
    data_store_handle.await.unwrap();
}

#[tokio::test]
async fn does_not_send_messages_without_import() {
    let (task_handle, mut test_handler, mut network) = prepare_data_store();
    let data_store_handle = tokio::spawn(task_handle);

    let blocks = test_handler.build_and_import_blocks(4, true).await;

    let not_imported_block = test_handler.build_block();

    test_handler.send_data(vec![AlephData::from(not_imported_block)]);

    test_handler.send_data(blocks.clone());

    let message = timeout(DEFAULT_TIMEOUT, network.next())
        .await
        .ok()
        .flatten()
        .expect("Did not receive message from Data Store");
    assert_eq!(message.included_blocks(), blocks);

    test_handler.exit();

    let message = network.next().await;
    assert!(message.is_none());

    data_store_handle.await.unwrap();
}

#[tokio::test]
async fn sends_block_request_on_missing_block() {
    let (task_handle, mut test_handler, _network) = prepare_data_store();
    let data_store_handle = tokio::spawn(task_handle);

    let data = AlephData::from(test_handler.build_block());

    test_handler.send_data(vec![data]);

    let requested_block = timeout(DEFAULT_TIMEOUT, test_handler.next_block_request())
        .await
        .expect("Did not receive block request from Data Store");
    assert_eq!(requested_block, data);

    test_handler.exit();
    data_store_handle.await.unwrap();
}
