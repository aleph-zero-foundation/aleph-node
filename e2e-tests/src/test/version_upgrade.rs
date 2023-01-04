use aleph_client::{
    pallets::{aleph::AlephSudoApi, elections::ElectionsApi, session::SessionApi},
    utility::BlocksApi,
    waiting::{AlephWaiting, BlockStatus},
    TxStatus,
};
use anyhow::anyhow;
use primitives::SessionIndex;

use crate::config::setup_test;

const UPGRADE_TO_VERSION: u32 = 1;

const UPGRADE_SESSION: SessionIndex = 3;

const UPGRADE_FINALIZATION_WAIT_SESSIONS: u32 = 3;

// Simple test that schedules a version upgrade, awaits it, and checks if node is still finalizing after planned upgrade session.
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

// A test that schedules a version upgrade which is supposed to fail, awaits it, and checks if finalization stopped.
// It's up to the user of this test to ensure that version upgrade will actually break finalization (non-compatible change in protocol, # updated nodes k is f < k < 2/3n).
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

    let connection = connection;
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
