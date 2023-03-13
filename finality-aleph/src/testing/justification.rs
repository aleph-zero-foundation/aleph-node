use std::{cell::RefCell, collections::VecDeque, time::Duration};

use aleph_primitives::BlockNumber;
use futures::{
    channel::mpsc::{unbounded, UnboundedSender},
    Future,
};
use sp_api::BlockId;
use sp_runtime::traits::Block;
use tokio::{task::JoinHandle, time::timeout};
use AcceptancePolicy::*;

use crate::{
    justification::{AlephJustification, JustificationHandler, JustificationHandlerConfig},
    testing::mocks::{
        create_block, AcceptancePolicy, Backend, JustificationRequestSchedulerImpl,
        MockedBlockFinalizer, MockedBlockRequester, SessionInfoProviderImpl, TBlock,
        VerifierWrapper,
    },
    JustificationNotification, SessionPeriod, SignatureSet,
};

const SESSION_PERIOD: SessionPeriod = SessionPeriod(5u32);
const FINALIZED_HEIGHT: BlockNumber = 22;

type TJustHandler = JustificationHandler<
    TBlock,
    VerifierWrapper,
    MockedBlockRequester,
    JustificationRequestSchedulerImpl,
    SessionInfoProviderImpl,
    MockedBlockFinalizer,
    Backend,
>;
type Sender = UnboundedSender<JustificationNotification<TBlock>>;
type Environment = (
    TJustHandler,
    Backend,
    MockedBlockRequester,
    MockedBlockFinalizer,
    JustificationRequestSchedulerImpl,
);

fn create_justification_notification_for(block: TBlock) -> JustificationNotification<TBlock> {
    JustificationNotification {
        justification: AlephJustification::CommitteeMultisignature(SignatureSet::with_size(
            0.into(),
        )),
        hash: block.hash(),
        number: block.header.number,
    }
}

fn run_justification_handler(
    justification_handler: TJustHandler,
) -> (JoinHandle<()>, Sender, Sender) {
    let (auth_just_tx, auth_just_rx) = unbounded();
    let (imp_just_tx, imp_just_rx) = unbounded();

    let handle =
        tokio::spawn(async move { justification_handler.run(auth_just_rx, imp_just_rx).await });

    (handle, auth_just_tx, imp_just_tx)
}

fn prepare_env(
    finalization_height: BlockNumber,
    verification_policy: AcceptancePolicy,
    request_policy: AcceptancePolicy,
) -> Environment {
    let backend = Backend::new(finalization_height);
    let info_provider = SessionInfoProviderImpl::new(SESSION_PERIOD, verification_policy);
    let finalizer = MockedBlockFinalizer::new();
    let requester = MockedBlockRequester::new();
    let config = JustificationHandlerConfig::test();
    let justification_request_scheduler = JustificationRequestSchedulerImpl::new(request_policy);

    let justification_handler = JustificationHandler::new(
        info_provider,
        requester.clone(),
        backend.clone(),
        finalizer.clone(),
        justification_request_scheduler.clone(),
        None,
        config,
    );

    (
        justification_handler,
        backend,
        requester,
        finalizer,
        justification_request_scheduler,
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn panics_and_stops_when_authority_channel_is_closed() {
    let justification_handler = prepare_env(1, AlwaysReject, AlwaysReject).0;
    let (handle, auth_just_tx, _) = run_justification_handler(justification_handler);
    auth_just_tx.close_channel();

    let handle = async move { handle.await.unwrap_err() };
    match timeout(Duration::from_millis(50), handle).await {
        Ok(err) => assert!(err.is_panic()),
        Err(_) => panic!("JustificationHandler did not stop!"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn panics_and_stops_when_import_channel_is_closed() {
    let justification_handler = prepare_env(1, AlwaysReject, AlwaysReject).0;
    let (handle, _, imp_just_tx) = run_justification_handler(justification_handler);
    imp_just_tx.close_channel();

    let handle = async move { handle.await.unwrap_err() };
    match timeout(Duration::from_millis(50), handle).await {
        Ok(err) => assert!(err.is_panic()),
        Err(_) => panic!("JustificationHandler did not stop!"),
    }
}

async fn run_test<F, S>(env: Environment, scenario: S)
where
    F: Future,
    S: FnOnce(
        Sender,
        Sender,
        Backend,
        MockedBlockRequester,
        MockedBlockFinalizer,
        JustificationRequestSchedulerImpl,
    ) -> F,
{
    let (justification_handler, backend, requester, finalizer, justification_request_scheduler) =
        env;
    let (handle_run, auth_just_tx, imp_just_tx) = run_justification_handler(justification_handler);
    scenario(
        auth_just_tx.clone(),
        imp_just_tx.clone(),
        backend,
        requester,
        finalizer,
        justification_request_scheduler,
    )
    .await;
    auth_just_tx.close_channel();
    imp_just_tx.close_channel();
    let _ = timeout(Duration::from_millis(10), handle_run).await;
}

async fn expect_finalized(
    finalizer: &MockedBlockFinalizer,
    justification_request_scheduler: &JustificationRequestSchedulerImpl,
    block: TBlock,
) {
    assert!(finalizer.has_been_invoked_with(block).await);
    assert!(justification_request_scheduler.has_been_finalized().await);
}

async fn expect_not_finalized(
    finalizer: &MockedBlockFinalizer,
    justification_request_scheduler: &JustificationRequestSchedulerImpl,
) {
    assert!(finalizer.has_not_been_invoked().await);
    assert!(!justification_request_scheduler.has_been_finalized().await);
}

async fn expect_requested(
    requester: &MockedBlockRequester,
    justification_request_scheduler: &JustificationRequestSchedulerImpl,
    block: TBlock,
) {
    assert!(requester.has_been_invoked_with(block).await);
    assert!(justification_request_scheduler.has_been_requested().await);
}

async fn expect_not_requested(
    requester: &MockedBlockRequester,
    justification_request_scheduler: &JustificationRequestSchedulerImpl,
) {
    assert!(requester.has_not_been_invoked().await);
    assert!(!justification_request_scheduler.has_been_requested().await);
}

#[tokio::test(flavor = "multi_thread")]
async fn leads_to_finalization_when_appropriate_justification_comes() {
    run_test(
        prepare_env(FINALIZED_HEIGHT, AlwaysAccept, AlwaysReject),
        |_, imp_just_tx, backend, _, finalizer, justification_request_scheduler| async move {
            let block = backend.next_block_to_finalize();
            let message = create_justification_notification_for(block.clone());
            imp_just_tx.unbounded_send(message).unwrap();
            expect_finalized(&finalizer, &justification_request_scheduler, block).await;
        },
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn waits_for_verifier_before_finalizing() {
    let verification_policy = FromSequence(RefCell::new(VecDeque::from(vec![false, false, true])));
    run_test(
        prepare_env(FINALIZED_HEIGHT, verification_policy, AlwaysReject),
        |_, imp_just_tx, backend, _, finalizer, justification_request_scheduler| async move {
            let block = backend.next_block_to_finalize();
            let message = create_justification_notification_for(block.clone());

            imp_just_tx.unbounded_send(message.clone()).unwrap();
            expect_not_finalized(&finalizer, &justification_request_scheduler).await;

            imp_just_tx.unbounded_send(message.clone()).unwrap();
            expect_not_finalized(&finalizer, &justification_request_scheduler).await;

            imp_just_tx.unbounded_send(message).unwrap();
            expect_finalized(&finalizer, &justification_request_scheduler, block).await;
        },
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn keeps_finalizing_block_if_not_finalized_yet() {
    run_test(
        prepare_env(FINALIZED_HEIGHT, AlwaysAccept, AlwaysReject),
        |auth_just_tx, imp_just_tx, backend, _, finalizer, justification_request_scheduler| async move {
            let block = backend.next_block_to_finalize();
            let message = create_justification_notification_for(block.clone());

            imp_just_tx.unbounded_send(message.clone()).unwrap();
            expect_finalized(&finalizer, &justification_request_scheduler, block.clone()).await;

            auth_just_tx.unbounded_send(message).unwrap();
            expect_finalized(&finalizer, &justification_request_scheduler, block).await;
        },
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn ignores_notifications_for_old_blocks() {
    run_test(
        prepare_env(FINALIZED_HEIGHT, AlwaysAccept, AlwaysReject),
        |_, imp_just_tx, backend, _, finalizer, justification_request_scheduler| async move {
            let block = backend.get_block(BlockId::Number(1)).unwrap();
            let message = create_justification_notification_for(block);
            imp_just_tx.unbounded_send(message).unwrap();
            expect_not_finalized(&finalizer, &justification_request_scheduler).await;
        },
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn ignores_notifications_from_future_session() {
    run_test(
        prepare_env(FINALIZED_HEIGHT, AlwaysAccept, AlwaysReject),
        |_, imp_just_tx, _, _, finalizer, justification_request_scheduler| async move {
            let block = create_block([1u8; 32].into(), FINALIZED_HEIGHT + SESSION_PERIOD.0);
            let message = create_justification_notification_for(block);
            imp_just_tx.unbounded_send(message).unwrap();
            expect_not_finalized(&finalizer, &justification_request_scheduler).await;
        },
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn does_not_buffer_notifications_from_future_session() {
    run_test(
        prepare_env(SESSION_PERIOD.0 - 2, AlwaysAccept, AlwaysReject),
        |_, imp_just_tx, backend, _, finalizer, justification_request_scheduler| async move {
            let current_block = backend.next_block_to_finalize();
            let future_block = create_block(current_block.hash(), SESSION_PERIOD.0);

            let message = create_justification_notification_for(future_block);
            imp_just_tx.unbounded_send(message).unwrap();
            expect_not_finalized(&finalizer, &justification_request_scheduler).await;

            let message = create_justification_notification_for(current_block.clone());
            imp_just_tx.unbounded_send(message).unwrap();
            expect_finalized(&finalizer, &justification_request_scheduler, current_block).await;

            expect_not_finalized(&finalizer, &justification_request_scheduler).await;
        },
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn requests_for_session_ending_justification() {
    run_test(
        prepare_env(SESSION_PERIOD.0 - 2, AlwaysReject, AlwaysAccept),
        |_, imp_just_tx, backend, requester, _, justification_request_scheduler| async move {
            let last_block = backend.next_block_to_finalize();

            // doesn't need any notification passed to keep asking
            expect_requested(
                &requester,
                &justification_request_scheduler,
                last_block.clone(),
            )
            .await;
            expect_requested(
                &requester,
                &justification_request_scheduler,
                last_block.clone(),
            )
            .await;

            // asks also after processing some notifications
            let message = create_justification_notification_for(last_block.clone());
            imp_just_tx.unbounded_send(message).unwrap();

            expect_requested(&requester, &justification_request_scheduler, last_block).await;
        },
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn does_not_request_for_session_ending_justification_too_often() {
    run_test(
        prepare_env(SESSION_PERIOD.0 - 2, AlwaysReject, AlwaysReject),
        |_, _, backend, requester, _, justification_request_scheduler| async move {
            expect_not_requested(&requester, &justification_request_scheduler).await;

            justification_request_scheduler.update_policy(AlwaysAccept);
            expect_requested(
                &requester,
                &justification_request_scheduler,
                backend.next_block_to_finalize(),
            )
            .await;

            justification_request_scheduler.update_policy(AlwaysReject);
            expect_not_requested(&requester, &justification_request_scheduler).await;
        },
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn does_not_request_nor_finalize_when_verifier_is_not_available() {
    run_test(
        prepare_env(SESSION_PERIOD.0 - 2, Unavailable, AlwaysAccept),
        |_, imp_just_tx, backend, requester, finalizer, justification_request_scheduler| async move {
            expect_not_requested(&requester, &justification_request_scheduler).await;

            let block = backend.next_block_to_finalize();
            imp_just_tx
                .unbounded_send(create_justification_notification_for(block))
                .unwrap();

            expect_not_finalized(&finalizer, &justification_request_scheduler).await;
            expect_not_requested(&requester, &justification_request_scheduler).await;
        },
    )
    .await;
}
