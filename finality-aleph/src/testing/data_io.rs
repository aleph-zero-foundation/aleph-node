use crate::data_io::{AlephNetworkMessage, DataStore};
use futures::{
    channel::{
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
    StreamExt,
};
use sc_block_builder::BlockBuilderProvider;
use sp_consensus::BlockOrigin;
use std::{future::Future, sync::Arc};
use substrate_test_runtime_client::{
    runtime::{Block, Hash},
    Backend, ClientBlockImportExt, DefaultTestClientBuilderExt, TestClient, TestClientBuilder,
    TestClientBuilderExt,
};

#[derive(Debug)]
struct TestNetworkData {
    data: Vec<Hash>,
}

impl AlephNetworkMessage<Block> for TestNetworkData {
    fn included_blocks(&self) -> Vec<Hash> {
        self.data.clone()
    }
}

fn prepare_data_store() -> (
    impl Future<Output = ()>,
    Arc<TestClient>,
    UnboundedSender<TestNetworkData>,
    UnboundedReceiver<TestNetworkData>,
    oneshot::Sender<()>,
) {
    let client = Arc::new(TestClientBuilder::new().build());

    let (aleph_network_tx, data_store_rx) = mpsc::unbounded();
    let (data_store_tx, aleph_network_rx) = mpsc::unbounded();
    let mut data_store = DataStore::<Block, TestClient, Backend, TestNetworkData>::new(
        client.clone(),
        data_store_tx,
        data_store_rx,
    );
    let (exit_data_store_tx, exit_data_store_rx) = oneshot::channel();

    (
        async move {
            data_store.run(exit_data_store_rx).await;
        },
        client,
        aleph_network_tx,
        aleph_network_rx,
        exit_data_store_tx,
    )
}

async fn import_blocks(client: &mut Arc<TestClient>, n: u32) -> Vec<Hash> {
    let mut blocks = vec![];
    for _ in 0..n {
        let block = client
            .new_block(Default::default())
            .unwrap()
            .build()
            .unwrap()
            .block;
        client
            .import(BlockOrigin::Own, block.clone())
            .await
            .unwrap();
        blocks.push(block.header.hash());
    }
    blocks
}

#[tokio::test]
async fn sends_messages_with_imported_blocks() {
    let (task_handle, mut client, store_tx, mut store_rx, exit_data_store_tx) =
        prepare_data_store();

    let data_store_handle = tokio::spawn(task_handle);

    let blocks = import_blocks(&mut client, 4).await;

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
    let (task_handle, mut client, store_tx, mut store_rx, exit_data_store_tx) =
        prepare_data_store();

    let data_store_handle = tokio::spawn(task_handle);

    let block = client
        .new_block(Default::default())
        .unwrap()
        .build()
        .unwrap()
        .block;
    store_tx
        .unbounded_send(TestNetworkData {
            data: vec![block.header.hash()],
        })
        .unwrap();
    client
        .import(BlockOrigin::Own, block.clone())
        .await
        .unwrap();

    let message = store_rx.next().await.expect("We own the tx");
    assert_eq!(message.included_blocks(), vec![block.header.hash()]);

    exit_data_store_tx.send(()).unwrap();
    data_store_handle.await.unwrap();
}

#[tokio::test]
async fn does_not_send_messages_without_import() {
    let (task_handle, mut client, store_tx, mut store_rx, exit_data_store_tx) =
        prepare_data_store();

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
            data: vec![not_imported_block.header.hash()],
        })
        .unwrap();

    store_tx
        .unbounded_send(TestNetworkData {
            data: vec![imported_block.header.hash()],
        })
        .unwrap();

    let message = store_rx.next().await.expect("We own the tx");
    assert_eq!(
        message.included_blocks(),
        vec![imported_block.header.hash()]
    );

    let message = store_rx.try_next();
    assert!(message.is_err());

    exit_data_store_tx.send(()).unwrap();
    data_store_handle.await.unwrap();
}
