use aleph_client::{
    pallets::{aleph::AlephSudoApi, elections::ElectionsApi, session::SessionApi},
    utility::BlocksApi,
    waiting::{AlephWaiting, BlockStatus},
    AsConnection, TxStatus,
};
use anyhow::anyhow;
use log::info;
use primitives::{BlockNumber, SessionIndex, Version, LEGACY_FINALITY_VERSION};

use crate::{
    config::setup_test,
    finality_version::{
        check_finality_version_at_block, check_next_session_finality_version_at_block,
    },
};

const UPGRADE_TO_VERSION: u32 = 1;
const UPGRADE_SESSION: SessionIndex = 3;
const UPGRADE_FINALIZATION_WAIT_SESSIONS: u32 = 3;

const SESSION_WITH_FINALITY_VERSION_CHANGE: SessionIndex = 4;
const SCHEDULING_OFFSET_SESSIONS: f64 = -2.5;
const CHECK_START_BLOCK: BlockNumber = 0;

/// Simple test that schedules a version upgrade, awaits it, and checks if the node is still finalizing after the planned upgrade session.
#[tokio::test]
pub async fn schedule_version_change() -> anyhow::Result<()> {
    let config = setup_test();
    let connection = config.create_root_connection().await;
    let test_case_params = config.test_case_params.clone();

    let current_session = connection.get_session(None).await;
    let version_for_upgrade = test_case_params
        .upgrade_to_version
        .unwrap_or(UPGRADE_TO_VERSION);
    let session_for_upgrade =
        current_session + test_case_params.upgrade_session.unwrap_or(UPGRADE_SESSION);
    let wait_sessions_after_upgrade = test_case_params
        .upgrade_finalization_wait_sessions
        .unwrap_or(UPGRADE_FINALIZATION_WAIT_SESSIONS);
    let session_after_upgrade = session_for_upgrade + wait_sessions_after_upgrade;

    connection
        .schedule_finality_version_change(
            version_for_upgrade,
            session_for_upgrade,
            TxStatus::Finalized,
        )
        .await?;
    connection
        .wait_for_session(session_after_upgrade + 1, BlockStatus::Finalized)
        .await;

    let block_number = connection
        .get_best_block()
        .await?
        .ok_or(anyhow!("Failed to retrieve best block number!"))?;
    connection
        .wait_for_block(|n| n >= block_number, BlockStatus::Finalized)
        .await;

    Ok(())
}

/// A test that schedules a version upgrade which is supposed to fail, awaits it, and checks if finalization stopped.
/// It's up to the user of this test to ensure that version upgrade will actually break finalization (a non-compatible change in protocol, # updated nodes k is f < k < 2/3n).
#[tokio::test]
pub async fn schedule_doomed_version_change_and_verify_finalization_stopped() -> anyhow::Result<()>
{
    let config = setup_test();
    let connection = config.create_root_connection().await;
    let test_case_params = config.test_case_params.clone();

    let current_session = connection.get_session(None).await;
    let version_for_upgrade = test_case_params
        .upgrade_to_version
        .unwrap_or(UPGRADE_TO_VERSION);
    let session_for_upgrade =
        current_session + test_case_params.upgrade_session.unwrap_or(UPGRADE_SESSION);
    let wait_sessions_after_upgrade = test_case_params
        .upgrade_finalization_wait_sessions
        .unwrap_or(UPGRADE_FINALIZATION_WAIT_SESSIONS);
    let session_after_upgrade = session_for_upgrade + wait_sessions_after_upgrade;

    connection
        .schedule_finality_version_change(
            version_for_upgrade,
            session_for_upgrade,
            TxStatus::Finalized,
        )
        .await?;
    connection
        .wait_for_session(session_after_upgrade + 1, BlockStatus::Best)
        .await;

    let last_finalized_block = session_for_upgrade * connection.get_session_period().await? - 1;

    let finalized_block_head = connection.get_finalized_block_hash().await?;
    let finalized_block = connection.get_block_number(finalized_block_head).await?;

    let finalized_block = match finalized_block {
        Some(block) => block,
        _ => {
            return Err(anyhow::Error::msg(
                "somehow no block was finalized (even though we saw one)",
            ))
        }
    };

    // check if finalization is still behind the upgrade-session
    assert!(finalized_block <= last_finalized_block);

    Ok(())
}

/// Sets up the test. Waits for block 2.5 sessions ahead of `SESSION_WITH_FINALITY_VERSION_CHANGE`.
/// Schedules a finality version change. Waits for all blocks of session
/// `SESSION_WITH_FINALITY_VERSION_CHANGE` to be finalized. Checks the finality version and the
/// finality version for the next session for all the blocks from block `CHECK_START_BLOCK`
/// until the end of session `SESSION_WITH_FINALITY_VERSION_CHANGE`.
#[tokio::test]
pub async fn finality_version_change() -> anyhow::Result<()> {
    let config = setup_test();
    let root_connection = config.create_root_connection().await;
    let session_period = root_connection.get_session_period().await?;

    let start_point_in_sessions =
        SESSION_WITH_FINALITY_VERSION_CHANGE as f64 + SCHEDULING_OFFSET_SESSIONS;
    let scheduling_block = (start_point_in_sessions * session_period as f64) as u32;
    let end_block = (SESSION_WITH_FINALITY_VERSION_CHANGE + 1) * session_period - 1;

    let first_incoming_finality_version = LEGACY_FINALITY_VERSION as Version + 1;

    info!(
        "Finality version check | start block: {} | end block: {}",
        CHECK_START_BLOCK, end_block,
    );
    info!(
        "Version change to be scheduled on block {} for block {}",
        scheduling_block,
        SESSION_WITH_FINALITY_VERSION_CHANGE * session_period
    );
    root_connection
        .wait_for_block(|n| n >= scheduling_block, BlockStatus::Finalized)
        .await;

    root_connection
        .schedule_finality_version_change(
            first_incoming_finality_version,
            SESSION_WITH_FINALITY_VERSION_CHANGE,
            TxStatus::Finalized,
        )
        .await?;

    root_connection
        .wait_for_block(|n| n >= end_block, BlockStatus::Finalized)
        .await;

    let finality_change_block = SESSION_WITH_FINALITY_VERSION_CHANGE * session_period;
    let last_block_with_default_next_session_finality_version =
        finality_change_block - session_period - 1;

    info!(
        "Checking default finality versions. Blocks {} to {}",
        CHECK_START_BLOCK, last_block_with_default_next_session_finality_version
    );
    for block in CHECK_START_BLOCK..(last_block_with_default_next_session_finality_version + 1) {
        check_finality_version_at_block(
            root_connection.as_connection(),
            block,
            LEGACY_FINALITY_VERSION as Version,
        )
        .await;
        check_next_session_finality_version_at_block(
            root_connection.as_connection(),
            block,
            LEGACY_FINALITY_VERSION as Version,
        )
        .await;
    }

    info!(
        "Checking finality versions for session prior to the change. Blocks {} to {}",
        last_block_with_default_next_session_finality_version + 1,
        finality_change_block - 1
    );
    for block in (last_block_with_default_next_session_finality_version + 1)..finality_change_block
    {
        check_finality_version_at_block(
            root_connection.as_connection(),
            block,
            LEGACY_FINALITY_VERSION as Version,
        )
        .await;
        check_next_session_finality_version_at_block(
            root_connection.as_connection(),
            block,
            first_incoming_finality_version,
        )
        .await;
    }
    info!(
        "Checking finality versions once the change has come into effect. Blocks {} to {}",
        finality_change_block, end_block
    );
    for block in finality_change_block..(end_block + 1) {
        check_finality_version_at_block(
            root_connection.as_connection(),
            block,
            first_incoming_finality_version,
        )
        .await;
        check_next_session_finality_version_at_block(
            root_connection.as_connection(),
            block,
            first_incoming_finality_version,
        )
        .await;
    }

    Ok(())
}
