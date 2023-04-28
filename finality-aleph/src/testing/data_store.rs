use std::{future::Future, sync::Arc, time::Duration};

use aleph_primitives::BlockNumber;
use futures::{
    channel::{
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
    StreamExt,
};
use sp_core::hash::H256;
use sp_runtime::traits::Block as BlockT;
use tokio::time::timeout;

use crate::{
    data_io::{AlephData, AlephNetworkMessage, DataStore, DataStoreConfig, MAX_DATA_BRANCH_LEN},
    network::{
        data::{component::Network as ComponentNetwork, Network as DataNetwork},
        Data, RequestBlocks,
    },
    session::{SessionBoundaries, SessionBoundaryInfo, SessionId, SessionPeriod},
    testing::{
        client_chain_builder::ClientChainBuilder,
        mocks::{
            aleph_data_from_blocks, aleph_data_from_headers, TBlock, THeader, TestClientBuilder,
            TestClientBuilderExt,
        },
    },
    IdentifierFor, Recipient,
};

#[derive(Clone)]
struct TestBlockRequester {
    blocks: UnboundedSender<IdentifierFor<TBlock>>,
    justifications: UnboundedSender<IdentifierFor<TBlock>>,
}

impl TestBlockRequester {
    fn new() -> (
        Self,
        UnboundedReceiver<IdentifierFor<TBlock>>,
        UnboundedReceiver<IdentifierFor<TBlock>>,
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

impl RequestBlocks<IdentifierFor<TBlock>> for TestBlockRequester {
    fn request_justification(&self, block_id: IdentifierFor<TBlock>) {
        self.justifications.unbounded_send(block_id).unwrap();
    }

    fn request_stale_block(&self, block_id: IdentifierFor<TBlock>) {
        self.blocks.unbounded_send(block_id).unwrap();
    }

    fn clear_justification_requests(&self) {
        panic!("`clear_justification_requests` not implemented!")
    }
}

type TestData = Vec<AlephData<TBlock>>;

impl AlephNetworkMessage<TBlock> for TestData {
    fn included_data(&self) -> Vec<AlephData<TBlock>> {
        self.clone()
    }
}

struct TestComponentNetwork<S, R> {
    sender: mpsc::UnboundedSender<(S, Recipient)>,
    receiver: mpsc::UnboundedReceiver<R>,
}

impl<D: Data> ComponentNetwork<D> for TestComponentNetwork<D, D> {
    type S = mpsc::UnboundedSender<(D, Recipient)>;
    type R = mpsc::UnboundedReceiver<D>;

    fn into(self) -> (Self::S, Self::R) {
        (self.sender, self.receiver)
    }
}

struct TestHandler {
    chain_builder: ClientChainBuilder,
    block_requests_rx: UnboundedReceiver<IdentifierFor<TBlock>>,
    justification_requests_rx: UnboundedReceiver<IdentifierFor<TBlock>>,
    network_tx: UnboundedSender<TestData>,
    network: Box<dyn DataNetwork<TestData>>,
}

impl TestHandler {
    /// Finalize block with given hash without providing justification.
    fn finalize_block(&self, hash: &H256) {
        self.chain_builder.finalize_block(hash);
    }

    fn genesis_hash(&mut self) -> H256 {
        self.chain_builder.genesis_hash()
    }

    fn get_header_at(&self, num: BlockNumber) -> THeader {
        self.chain_builder.get_header_at(num)
    }

    async fn build_block_above(&mut self, parent: &H256) -> TBlock {
        self.chain_builder.build_block_above(parent).await
    }

    /// Builds a sequence of blocks extending from `hash` of length `len`
    async fn build_branch_above(&mut self, parent: &H256, len: usize) -> Vec<TBlock> {
        self.chain_builder.build_branch_above(parent, len).await
    }

    /// imports a sequence of blocks, should be in correct order
    async fn import_branch(&mut self, blocks: Vec<TBlock>) {
        self.chain_builder.import_branch(blocks).await;
    }

    /// Builds and imports a sequence of blocks extending from genesis of length `len`
    async fn initialize_single_branch_and_import(&mut self, len: usize) -> Vec<TBlock> {
        self.chain_builder
            .initialize_single_branch_and_import(len)
            .await
    }

    /// Builds a sequence of blocks extending from genesis of length `len`
    async fn initialize_single_branch(&mut self, len: usize) -> Vec<TBlock> {
        self.chain_builder.initialize_single_branch(len).await
    }

    /// Sends data to Data Store
    fn send_data(&self, data: TestData) {
        self.network_tx.unbounded_send(data).unwrap()
    }

    /// Receive next block request from Data Store
    async fn next_block_request(&mut self) -> IdentifierFor<TBlock> {
        self.block_requests_rx.next().await.unwrap()
    }

    /// Receive next justification request from Data Store
    async fn next_justification_request(&mut self) -> IdentifierFor<TBlock> {
        self.justification_requests_rx.next().await.unwrap()
    }

    async fn assert_no_message_out(&mut self, err_message: &'static str) {
        let res = timeout(TIMEOUT_FAIL, self.network.next()).await;
        assert!(res.is_err(), "{} (message out: {:?})", err_message, res);
    }

    async fn assert_message_out(&mut self, err_message: &'static str) -> TestData {
        timeout(TIMEOUT_SUCC, self.network.next())
            .await
            .expect(err_message)
            .expect(err_message)
    }
}

fn prepare_data_store(
    session_boundaries: Option<SessionBoundaries>,
) -> (impl Future<Output = ()>, oneshot::Sender<()>, TestHandler) {
    let client = Arc::new(TestClientBuilder::new().build());

    let (block_requester, block_requests_rx, justification_requests_rx) = TestBlockRequester::new();
    let (sender_tx, _sender_rx) = mpsc::unbounded();
    let (network_tx, network_rx) = mpsc::unbounded();
    let test_network = TestComponentNetwork {
        sender: sender_tx,
        receiver: network_rx,
    };
    let data_store_config = DataStoreConfig {
        max_triggers_pending: 80_000,
        max_proposals_pending: 80_000,
        max_messages_pending: 40_000,
        available_proposals_cache_capacity: 8000,
        periodic_maintenance_interval: Duration::from_millis(20),
        request_block_after: Duration::from_millis(30),
    };

    let session_boundaries = if let Some(session_boundaries) = session_boundaries {
        session_boundaries
    } else {
        SessionBoundaryInfo::new(SessionPeriod(900)).boundaries_for_session(SessionId(0))
    };
    let (mut data_store, network) = DataStore::new(
        session_boundaries,
        client.clone(),
        block_requester,
        data_store_config,
        test_network,
    );

    let chain_builder = ClientChainBuilder::new(client, Arc::new(TestClientBuilder::new().build()));
    let (exit_data_store_tx, exit_data_store_rx) = oneshot::channel();

    (
        async move {
            data_store.run(exit_data_store_rx).await;
        },
        exit_data_store_tx,
        TestHandler {
            chain_builder,
            block_requests_rx,
            justification_requests_rx,
            network_tx,
            network: Box::new(network),
        },
    )
}

const TIMEOUT_SUCC: Duration = Duration::from_millis(5000);
const TIMEOUT_FAIL: Duration = Duration::from_millis(200);

// This is the basic assumption for other tests, so we better test it, in case this somehow changes in the future.
#[tokio::test]
async fn forks_have_different_block_hashes() {
    let (_task_handle, _exit, mut test_handler) = prepare_data_store(None);
    let genesis_hash = test_handler.genesis_hash();
    let a1 = test_handler.build_block_above(&genesis_hash).await;
    let b1 = test_handler.build_block_above(&genesis_hash).await;
    assert_ne!(a1.hash(), b1.hash());
}

async fn run_test<F, S>(scenario: S)
where
    F: Future,
    S: FnOnce(TestHandler) -> F,
{
    let (task_handle, exit, test_handler) = prepare_data_store(None);
    let data_store_handle = tokio::spawn(task_handle);

    scenario(test_handler).await;

    exit.send(()).unwrap();
    data_store_handle.await.unwrap();
}

#[tokio::test]
async fn correct_messages_go_through() {
    run_test(|mut test_handler| async move {
        let blocks = test_handler
            .initialize_single_branch_and_import(MAX_DATA_BRANCH_LEN * 10)
            .await;

        for i in 1..=MAX_DATA_BRANCH_LEN {
            let blocks_branch = blocks[0..i].to_vec();
            let test_data: TestData = vec![aleph_data_from_blocks(blocks_branch)];
            test_handler.send_data(test_data.clone());

            let message = test_handler
                .assert_message_out("Did not receive message from Data Store")
                .await;
            assert_eq!(message.included_data(), test_data);
        }
    })
    .await;
}

#[tokio::test]
async fn too_long_branch_message_does_not_go_through() {
    run_test(|mut test_handler| async move {
        let blocks = test_handler
            .initialize_single_branch_and_import(MAX_DATA_BRANCH_LEN * 10)
            .await;

        test_handler.finalize_block(&blocks[MAX_DATA_BRANCH_LEN + 2].hash());

        let blocks_branch = blocks[0..(MAX_DATA_BRANCH_LEN + 1)].to_vec();
        let test_data: TestData = vec![aleph_data_from_blocks(blocks_branch)];
        test_handler.send_data(test_data.clone());
        test_handler
            .assert_no_message_out("Data Store let through a too long message")
            .await;
    })
    .await;
}

#[tokio::test]
async fn branch_not_within_session_boundaries_does_not_go_through() {
    let session_boundaries =
        SessionBoundaryInfo::new(SessionPeriod(20)).boundaries_for_session(SessionId(1));
    let session_start = session_boundaries.first_block() as usize;
    let session_end = session_boundaries.last_block() as usize;

    let (task_handle, exit, mut test_handler) = prepare_data_store(Some(session_boundaries));
    let data_store_handle = tokio::spawn(task_handle);
    let blocks = test_handler
        .initialize_single_branch_and_import(MAX_DATA_BRANCH_LEN * 10)
        .await;

    for boundary_point in &[session_start, session_end] {
        for l in 0..MAX_DATA_BRANCH_LEN {
            for r in 0..MAX_DATA_BRANCH_LEN {
                let left_end = boundary_point - l;
                let right_end = boundary_point + r;
                if right_end - left_end < MAX_DATA_BRANCH_LEN
                    && !(session_start <= left_end && right_end <= session_end)
                {
                    // blocks start from block num 1, as genesis is block 0, we need to shift the indexing
                    let blocks_branch = blocks[(left_end - 1)..right_end].to_vec();
                    test_handler.send_data(vec![aleph_data_from_blocks(blocks_branch)]);
                }
            }
        }
    }

    test_handler
        .assert_no_message_out("Data Store let through a message not within session_boundaries")
        .await;

    test_handler.finalize_block(&blocks[session_end + MAX_DATA_BRANCH_LEN].hash());

    test_handler
        .assert_no_message_out("Data Store let through a message not within session_boundaries")
        .await;

    for boundary_point in &[session_start, session_end] {
        for l in 0..MAX_DATA_BRANCH_LEN {
            for r in 0..MAX_DATA_BRANCH_LEN {
                let left_end = boundary_point - l;
                let right_end = boundary_point + r;
                if right_end - left_end < MAX_DATA_BRANCH_LEN
                    && session_start <= left_end
                    && right_end <= session_end
                {
                    // blocks start from block num 1, as genesis is block 0, we need to shift the indexing
                    let blocks_branch = blocks[(left_end - 1)..right_end].to_vec();
                    test_handler.send_data(vec![aleph_data_from_blocks(blocks_branch)]);
                    test_handler
                        .assert_message_out("Data Store held available proposal")
                        .await;
                }
            }
        }
    }

    exit.send(()).unwrap();
    data_store_handle.await.unwrap();
}

#[tokio::test]
async fn branch_with_not_finalized_ancestor_correctly_handled() {
    run_test(|mut test_handler| async move {
        let blocks = test_handler
            .initialize_single_branch_and_import(MAX_DATA_BRANCH_LEN * 10)
            .await;

        let blocks_branch = blocks[1..2].to_vec();
        let test_data: TestData = vec![aleph_data_from_blocks(blocks_branch)];
        test_handler.send_data(test_data.clone());

        test_handler
            .assert_no_message_out("Data Store let through a message with not finalized ancestor")
            .await;

        // After the block gets finalized the message should be let through
        test_handler.finalize_block(&blocks[0].hash());
        let message = test_handler
            .assert_message_out("Did not receive message from Data Store")
            .await;
        assert_eq!(message.included_data(), test_data);
    })
    .await;
}

fn send_proposals_of_each_len(blocks: Vec<TBlock>, test_handler: &mut TestHandler) {
    for i in 1..=MAX_DATA_BRANCH_LEN {
        let blocks_branch = blocks[0..i].to_vec();
        let test_data: TestData = vec![aleph_data_from_blocks(blocks_branch)];
        test_handler.send_data(test_data.clone());
    }
}

#[tokio::test]
async fn correct_messages_go_through_with_late_import() {
    run_test(|mut test_handler| async move {
        let blocks = test_handler
            .initialize_single_branch(MAX_DATA_BRANCH_LEN * 10)
            .await;

        send_proposals_of_each_len(blocks.clone(), &mut test_handler);

        test_handler
            .assert_no_message_out("Data Store let through a message with not yet imported blocks")
            .await;

        test_handler.import_branch(blocks).await;

        for _ in 1..=MAX_DATA_BRANCH_LEN {
            test_handler
                .assert_message_out("Did not receive message from Data Store")
                .await;
        }
    })
    .await;
}

#[tokio::test]
async fn message_with_multiple_data_gets_through_when_it_should() {
    run_test(|mut test_handler| async move {
        let max_height = MAX_DATA_BRANCH_LEN + 12;
        let blocks = test_handler
            .initialize_single_branch_and_import(max_height + 10 * MAX_DATA_BRANCH_LEN)
            .await;
        let mut test_data = vec![];
        for i in 1..=max_height {
            let blocks_branch = blocks[i..(i + 1)].to_vec();
            test_data.push(aleph_data_from_blocks(blocks_branch));
        }
        test_handler.send_data(test_data.clone());

        test_handler
            .assert_no_message_out("Data Store let through a message with not finalized ancestor")
            .await;

        test_handler.finalize_block(&blocks[max_height - 2].hash());
        // This should not be enough yet (ancestor not finalized for some data items)

        test_handler
            .assert_no_message_out("Data Store let through a message with not finalized ancestor")
            .await;

        test_handler.finalize_block(&blocks[max_height - 1].hash());

        test_handler
            .assert_message_out("Did not receive message from Data Store")
            .await;
    })
    .await;
}

#[tokio::test]
async fn sends_block_request_on_missing_block() {
    run_test(|mut test_handler| async move {
        let blocks = test_handler
            .initialize_single_branch(MAX_DATA_BRANCH_LEN * 10)
            .await;
        let blocks_branch = blocks[0..1].to_vec();
        let test_data: TestData = vec![aleph_data_from_blocks(blocks_branch)];
        test_handler.send_data(test_data.clone());

        test_handler
            .assert_no_message_out("Data Store let through a message with not imported block")
            .await;

        let requested_block = timeout(TIMEOUT_SUCC, test_handler.next_block_request())
            .await
            .expect("Did not receive block request from Data Store");
        assert_eq!(requested_block.hash, blocks[0].hash());

        test_handler.import_branch(blocks).await;

        test_handler
            .assert_message_out("Did not receive message from Data Store")
            .await;
    })
    .await;
}

#[tokio::test]
async fn sends_justification_request_when_not_finalized() {
    run_test(|mut test_handler| async move {
        let blocks = test_handler
            .initialize_single_branch(MAX_DATA_BRANCH_LEN * 10)
            .await;
        test_handler.import_branch(blocks.clone()).await;

        let blocks_branch = vec![blocks[2].clone()];
        let test_data = vec![aleph_data_from_blocks(blocks_branch)];
        test_handler.send_data(test_data);

        test_handler
            .assert_no_message_out(
                "Data Store let through a message with not finalized parent of base block",
            )
            .await;

        let requested_block = timeout(TIMEOUT_SUCC, test_handler.next_justification_request())
            .await
            .expect("Did not receive block request from Data Store");
        assert_eq!(requested_block.hash, blocks[1].hash());

        test_handler.finalize_block(&blocks[1].hash());

        test_handler
            .assert_message_out("Did not receive message from Data Store")
            .await;
    })
    .await;
}

#[tokio::test]
async fn does_not_send_requests_when_no_block_missing() {
    run_test(|mut test_handler| async move {
        let blocks = test_handler
            .initialize_single_branch_and_import(MAX_DATA_BRANCH_LEN * 10)
            .await;

        send_proposals_of_each_len(blocks, &mut test_handler);

        assert!(
            timeout(TIMEOUT_FAIL, test_handler.next_block_request())
                .await
                .is_err(),
            "Data Store is sending block requests without reason"
        );
    })
    .await;
}

#[tokio::test]
async fn message_with_genesis_block_does_not_get_through() {
    run_test(|mut test_handler| async move {
        let _ = test_handler
            .initialize_single_branch_and_import(MAX_DATA_BRANCH_LEN * 10)
            .await;

        for i in 1..MAX_DATA_BRANCH_LEN {
            let test_data: TestData = vec![aleph_data_from_headers(
                (0..i)
                    .into_iter()
                    .map(|num| test_handler.get_header_at(num as BlockNumber))
                    .collect(),
            )];
            test_handler.send_data(test_data.clone());
        }

        test_handler
            .assert_no_message_out("Data Store let through a message with genesis block proposal")
            .await;
    })
    .await;
}

#[tokio::test]
async fn unimported_hopeless_forks_go_through() {
    run_test(|mut test_handler| async move {
        let blocks = test_handler
            .initialize_single_branch(MAX_DATA_BRANCH_LEN * 10)
            .await;

        let forking_block = &blocks[MAX_DATA_BRANCH_LEN + 2];
        let fork = test_handler
            .build_branch_above(&forking_block.hash(), MAX_DATA_BRANCH_LEN * 10)
            .await;

        send_proposals_of_each_len(fork.clone(), &mut test_handler);

        test_handler
            .assert_no_message_out("Data Store let through a message with not yet imported blocks")
            .await;

        test_handler.import_branch(blocks.clone()).await;

        test_handler.finalize_block(&blocks[MAX_DATA_BRANCH_LEN + 2].hash());

        test_handler
        .assert_no_message_out(
            "Data Store let through a message with not yet imported and not hopeless fork blocks",
        )
        .await;

        test_handler.finalize_block(&blocks[MAX_DATA_BRANCH_LEN + 3].hash());

        for _ in 1..=MAX_DATA_BRANCH_LEN {
            test_handler
                .assert_message_out("Did not receive message from Data Store")
                .await;
        }
    })
    .await;
}

#[tokio::test]
async fn imported_hopeless_forks_go_through() {
    run_test(|mut test_handler| async move {
        let blocks = test_handler
            .initialize_single_branch(10 * MAX_DATA_BRANCH_LEN)
            .await;

        let forking_block = &blocks[MAX_DATA_BRANCH_LEN + 2];
        let fork = test_handler
            .build_branch_above(&forking_block.hash(), MAX_DATA_BRANCH_LEN * 10)
            .await;

        test_handler.import_branch(blocks.clone()).await;
        test_handler.import_branch(fork.clone()).await;

        send_proposals_of_each_len(fork.clone(), &mut test_handler);

        test_handler
            .assert_no_message_out(
                "Data Store let through a hopeless fork with not finalized ancestor",
            )
            .await;

        test_handler.finalize_block(&blocks[MAX_DATA_BRANCH_LEN + 1].hash());

        test_handler
            .assert_no_message_out(
                "Data Store let through a hopeless fork with not finalized ancestor",
            )
            .await;

        test_handler.finalize_block(&blocks[MAX_DATA_BRANCH_LEN * 2 + 1].hash());

        for _ in 1..=MAX_DATA_BRANCH_LEN {
            test_handler
                .assert_message_out("Did not receive message from Data Store")
                .await;
        }
    })
    .await;
}

#[tokio::test]
async fn hopeless_fork_at_the_boundary_goes_through() {
    run_test(|mut test_handler| async move {
        let blocks = test_handler
            .initialize_single_branch(10 * MAX_DATA_BRANCH_LEN)
            .await;
        let fork_num = MAX_DATA_BRANCH_LEN + 2;
        let forking_block = &blocks[fork_num];
        let fork = test_handler
            .build_branch_above(&forking_block.hash(), MAX_DATA_BRANCH_LEN * 10)
            .await;

        test_handler.import_branch(blocks.clone()).await;

        let honest_hopeless_fork = vec![
            blocks[fork_num - 2].clone(),
            blocks[fork_num - 1].clone(),
            blocks[fork_num].clone(),
            fork[0].clone(),
        ];
        let honest_hopeless_fork2 = vec![
            blocks[fork_num - 2].clone(),
            blocks[fork_num - 1].clone(),
            blocks[fork_num].clone(),
            fork[0].clone(),
            fork[1].clone(),
        ];
        let malicious_hopeless_fork = vec![
            blocks[fork_num - 2].clone(),
            blocks[fork_num - 1].clone(),
            blocks[fork_num].clone(),
            fork[1].clone(),
        ];
        let malicious_hopeless_fork2 = vec![
            blocks[fork_num - 2].clone(),
            blocks[fork_num - 1].clone(),
            blocks[fork_num].clone(),
            blocks[fork_num + 1].clone(),
            fork[1].clone(),
        ];
        let honest_data = vec![aleph_data_from_blocks(honest_hopeless_fork)];
        let honest_data2 = vec![aleph_data_from_blocks(honest_hopeless_fork2)];
        let malicious_data = vec![aleph_data_from_blocks(malicious_hopeless_fork)];
        let malicious_data2 = vec![aleph_data_from_blocks(malicious_hopeless_fork2)];

        test_handler.send_data(honest_data.clone());
        test_handler.send_data(honest_data2.clone());
        test_handler.send_data(malicious_data.clone());
        test_handler.send_data(malicious_data2.clone());

        test_handler
            .assert_no_message_out("Data Store let through a message with not yet imported blocks")
            .await;

        test_handler.finalize_block(&blocks[fork_num].hash());

        let message = test_handler
            .assert_message_out("Did not receive message from Data Store")
            .await;
        assert_eq!(message, malicious_data);

        test_handler
            .assert_no_message_out("Data Store let through a message with not yet imported blocks")
            .await;

        test_handler.finalize_block(&blocks[fork_num + 1].hash());

        test_handler
            .assert_message_out("Did not receive message from Data Store")
            .await;

        test_handler
            .assert_message_out("Did not receive message from Data Store")
            .await;

        test_handler
            .assert_no_message_out("Data Store let through a message with not yet imported blocks")
            .await;

        test_handler.finalize_block(&blocks[fork_num + 2].hash());

        let message = test_handler
            .assert_message_out("Did not receive message from Data Store")
            .await;
        assert_eq!(message, malicious_data2);
    })
    .await;
}
