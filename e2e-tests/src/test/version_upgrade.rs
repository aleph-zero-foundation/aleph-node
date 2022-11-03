use aleph_client::{
    get_current_session, get_session_period, schedule_upgrade, wait_for_at_least_session,
    wait_for_finalized_block, AnyConnection,
};
use primitives::{Header, SessionIndex};

use crate::Config;

const UPGRADE_TO_VERSION: u32 = 1;

const UPGRADE_SESSION: SessionIndex = 3;

const UPGRADE_FINALIZATION_WAIT_SESSIONS: u32 = 3;

// Simple test that schedules a version upgrade, awaits it, and checks if node is still finalizing after planned upgrade session.
pub fn schedule_version_change(config: &Config) -> anyhow::Result<()> {
    let connection = config.create_root_connection();
    let test_case_params = config.test_case_params.clone();

    let current_session = get_current_session(&connection);
    let version_for_upgrade = test_case_params
        .upgrade_to_version
        .unwrap_or(UPGRADE_TO_VERSION);
    let session_for_upgrade =
        current_session + test_case_params.upgrade_session.unwrap_or(UPGRADE_SESSION);
    let wait_sessions_after_upgrade = test_case_params
        .upgrade_finalization_wait_sessions
        .unwrap_or(UPGRADE_FINALIZATION_WAIT_SESSIONS);
    let session_after_upgrade = session_for_upgrade + wait_sessions_after_upgrade;

    schedule_upgrade(&connection, version_for_upgrade, session_for_upgrade)?;

    wait_for_at_least_session(&connection, session_after_upgrade)?;
    let block_number = session_after_upgrade * get_session_period(&connection);
    wait_for_finalized_block(&connection, block_number)?;

    Ok(())
}

// A test that schedules a version upgrade which is supposed to fail, awaits it, and checks if finalization stopped.
// It's up to the user of this test to ensure that version upgrade will actually break finalization (non-compatible change in protocol, # updated nodes k is f < k < 2/3n).
pub fn schedule_doomed_version_change_and_verify_finalization_stopped(
    config: &Config,
) -> anyhow::Result<()> {
    let connection = config.create_root_connection();
    let test_case_params = config.test_case_params.clone();

    let current_session = get_current_session(&connection);
    let version_for_upgrade = test_case_params
        .upgrade_to_version
        .unwrap_or(UPGRADE_TO_VERSION);
    let session_for_upgrade =
        current_session + test_case_params.upgrade_session.unwrap_or(UPGRADE_SESSION);
    let wait_sessions_after_upgrade = test_case_params
        .upgrade_finalization_wait_sessions
        .unwrap_or(UPGRADE_FINALIZATION_WAIT_SESSIONS);
    let session_after_upgrade = session_for_upgrade + wait_sessions_after_upgrade;

    schedule_upgrade(&connection, version_for_upgrade, session_for_upgrade)?;
    wait_for_at_least_session(&connection, session_for_upgrade)?;
    let last_finalized_block = session_for_upgrade * get_session_period(&connection) - 1;

    wait_for_at_least_session(&connection, session_after_upgrade)?;
    let connection = connection.as_connection();
    let finalized_block_head = connection.as_connection().get_finalized_head()?;
    let finalized_block = connection.get_header::<Header>(finalized_block_head)?;

    let finalized_block = match finalized_block {
        Some(block) => block.number,
        None => {
            return Err(anyhow::Error::msg(
                "somehow no block was finalized (even though we saw one)",
            ))
        }
    };

    // check if finalization is still behind the upgrade-session
    assert!(finalized_block <= last_finalized_block);

    Ok(())
}
