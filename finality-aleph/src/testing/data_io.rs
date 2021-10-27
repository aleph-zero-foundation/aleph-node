use crate::data_io::{AlephData, AlephDataFor, AlephNetworkMessage, DataStore, DataStoreConfig};
use crate::network::RequestBlocks;
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
use std::{future::Future, sync::Arc, time::Duration};
use substrate_test_runtime_client::{
    runtime::{Block, Hash},
    Backend, ClientBlockImportExt, DefaultTestClientBuilderExt, TestClient, TestClientBuilder,
    TestClientBuilderExt,
};

use std::default::Default;

#[derive(Clone)]
struct TestBlockRequester<B: BlockT> {
    blocks: mpsc::UnboundedSender<AlephDataFor<B>>,
    justifications: mpsc::UnboundedSender<AlephDataFor<B>>,
}

impl<B: BlockT> TestBlockRequester<B> {
    fn new() -> (
        Self,
        mpsc::UnboundedReceiver<AlephDataFor<B>>,
        mpsc::UnboundedReceiver<AlephDataFor<B>>,
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
}

#[derive(Debug)]
struct TestNetworkData {
    data: Vec<AlephDataFor<Block>>,
}

impl AlephNetworkMessage<Block> for TestNetworkData {
    fn included_blocks(&self) -> Vec<AlephDataFor<Block>> {
        self.data.clone()
    }
}

struct DataStoreChannels {
    store_tx: UnboundedSender<TestNetworkData>,
    store_rx: UnboundedReceiver<TestNetworkData>,
    block_requests_rx: UnboundedReceiver<AlephDataFor<Block>>,
    exit_data_store_tx: oneshot::Sender<()>,
}

fn prepare_data_store() -> (impl Future<Output = ()>, Arc<TestClient>, DataStoreChannels) {
    let client = Arc::new(TestClientBuilder::new().build());

    let (aleph_network_tx, data_store_rx) = mpsc::unbounded();
    let (data_store_tx, aleph_network_rx) = mpsc::unbounded();
    let (block_requester, block_requests_rx, _justification_requests_rx) =
        TestBlockRequester::new();

    let data_store_config = DataStoreConfig {
        available_blocks_cache_capacity: 1000,
        message_id_boundary: 100_000,
        periodic_maintenance_interval: Duration::from_millis(30),
        request_block_after: Duration::from_millis(50),
    };
    let mut data_store =
        DataStore::<Block, TestClient, Backend, TestBlockRequester<Block>, TestNetworkData>::new(
            client.clone(),
            block_requester,
            data_store_tx,
            data_store_rx,
            data_store_config,
        );
    let (exit_data_store_tx, exit_data_store_rx) = oneshot::channel();
    (
        async move {
            data_store.run(exit_data_store_rx).await;
        },
        client,
        DataStoreChannels {
            store_tx: aleph_network_tx,
            store_rx: aleph_network_rx,
            block_requests_rx,
            exit_data_store_tx,
        },
    )
}

async fn import_blocks(
    client: &mut Arc<TestClient>,
    n: u32,
    finalize: bool,
) -> Vec<AlephDataFor<Block>> {
    let mut blocks = vec![];
    for _ in 0..n {
        let block = client
            .new_block(Default::default())
            .unwrap()
            .build()
            .unwrap()
            .block;
        if finalize {
            client
                .import_as_final(BlockOrigin::Own, block.clone())
                .await
                .unwrap();
        } else {
            client
                .import(BlockOrigin::Own, block.clone())
                .await
                .unwrap();
        }
        blocks.push(AlephData::new(block.header.hash(), block.header.number));
    }
    blocks
}

#[tokio::test]
async fn sends_messages_with_imported_blocks() {
    let (
        task_handle,
        mut client,
        DataStoreChannels {
            store_tx,
            mut store_rx,
            block_requests_rx: _,
            exit_data_store_tx,
        },
    ) = prepare_data_store();

    let data_store_handle = tokio::spawn(task_handle);

    let blocks = import_blocks(&mut client, 4, false).await;

    store_tx
        .unbounded_send(TestNetworkData {
            data: blocks.clone(),
        })
        .unwrap();

    let message = store_rx.next().await.expect("We own the tx");
    assert_eq!(message.included_blocks(), blocks);

    exit_data_store_tx.send(()).unwrap();
    data_store_handle.await.unwrap();
}

#[tokio::test]
async fn sends_messages_after_import() {
    let (
        task_handle,
        mut client,
        DataStoreChannels {
            store_tx,
            mut store_rx,
            block_requests_rx: _,
            exit_data_store_tx,
        },
    ) = prepare_data_store();

    let data_store_handle = tokio::spawn(task_handle);

    let block = client
        .new_block(Default::default())
        .unwrap()
        .build()
        .unwrap()
        .block;

    let data = AlephData::new(block.header.hash(), block.header.number);

    store_tx
        .unbounded_send(TestNetworkData { data: vec![data] })
        .unwrap();
    client
        .import(BlockOrigin::Own, block.clone())
        .await
        .unwrap();

    let message = store_rx.next().await.expect("We own the tx");
    assert_eq!(message.included_blocks(), vec![data]);

    exit_data_store_tx.send(()).unwrap();
    data_store_handle.await.unwrap();
}

#[tokio::test]
async fn sends_messages_with_number_lower_than_finalized() {
    let (
        task_handle,
        mut client,
        DataStoreChannels {
            store_tx,
            mut store_rx,
            block_requests_rx: _,
            exit_data_store_tx,
        },
    ) = prepare_data_store();

    let data_store_handle = tokio::spawn(task_handle);

    import_blocks(&mut client, 4, true).await;

    let mut digest = Digest::default();
    digest.push(sp_runtime::generic::DigestItem::Other::<Hash>(
        1u32.to_le_bytes().to_vec(),
    ));

    let block = client
        .new_block_at(&BlockId::Number(1), digest, false)
        .unwrap()
        .build()
        .unwrap()
        .block;

    let data = AlephData::new(block.header.hash(), block.header.number);

    store_tx
        .unbounded_send(TestNetworkData { data: vec![data] })
        .unwrap();

    let message = store_rx.next().await.expect("We own the tx");
    assert_eq!(message.included_blocks(), vec![data]);

    exit_data_store_tx.send(()).unwrap();
    data_store_handle.await.unwrap();
}

#[tokio::test]
async fn does_not_send_messages_without_import() {
    let (
        task_handle,
        mut client,
        DataStoreChannels {
            store_tx,
            mut store_rx,
            block_requests_rx: _,
            exit_data_store_tx,
        },
    ) = prepare_data_store();

    let data_store_handle = tokio::spawn(task_handle);

    let imported_block = client
        .new_block(Default::default())
        .unwrap()
        .build()
        .unwrap()
        .block;

    client
        .import(BlockOrigin::Own, imported_block.clone())
        .await
        .unwrap();

    let not_imported_block = client
        .new_block(Default::default())
        .unwrap()
        .build()
        .unwrap()
        .block;

    store_tx
        .unbounded_send(TestNetworkData {
            data: vec![AlephData::new(
                not_imported_block.header.hash(),
                not_imported_block.header.number,
            )],
        })
        .unwrap();

    let data = AlephData::new(imported_block.header.hash(), imported_block.header.number);
    store_tx
        .unbounded_send(TestNetworkData { data: vec![data] })
        .unwrap();

    let message = store_rx.next().await.expect("We own the tx");
    assert_eq!(message.included_blocks(), vec![data]);

    let message = store_rx.try_next();
    assert!(message.is_err());

    exit_data_store_tx.send(()).unwrap();
    data_store_handle.await.unwrap();
}

#[tokio::test]
async fn sends_block_request_on_missing_block() {
    let (
        task_handle,
        client,
        DataStoreChannels {
            store_tx,
            store_rx: _,
            mut block_requests_rx,
            exit_data_store_tx,
        },
    ) = prepare_data_store();

    let data_store_handle = tokio::spawn(task_handle);

    let not_imported_block = client
        .new_block(Default::default())
        .unwrap()
        .build()
        .unwrap()
        .block;

    let data = AlephData::new(
        not_imported_block.header.hash(),
        not_imported_block.header.number,
    );
    store_tx
        .unbounded_send(TestNetworkData { data: vec![data] })
        .unwrap();

    let requested_block = block_requests_rx
        .next()
        .await
        .expect("Block should be requested");
    assert_eq!(requested_block, data);

    exit_data_store_tx.send(()).unwrap();
    data_store_handle.await.unwrap();
}
