use aleph_client::{
    get_current_session, get_session_period, schedule_upgrade, wait_for_at_least_session,
    wait_for_finalized_block,
};
use primitives::SessionIndex;

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
