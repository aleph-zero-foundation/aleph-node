use aleph_client::{
    get_current_era, get_current_era_validators, get_current_session, wait_for_at_least_session,
    SignedConnection,
};
use log::info;
use primitives::{
    BanInfo, BanReason, SessionCount, DEFAULT_BAN_MINIMAL_EXPECTED_PERFORMANCE,
    DEFAULT_BAN_SESSION_COUNT_THRESHOLD, DEFAULT_CLEAN_SESSION_COUNTER_DELAY,
};

use crate::{
    accounts::{get_validator_seed, NodeKeys},
    ban::{
        check_ban_config, check_ban_event, check_underperformed_validator_reason,
        check_underperformed_validator_session_count, check_validators, setup_test,
    },
    rewards::set_invalid_keys_for_validator,
    Config,
};

const VALIDATOR_TO_DISABLE_NON_RESERVED_INDEX: u32 = 0;
const VALIDATOR_TO_DISABLE_OVERALL_INDEX: u32 = 2;
// Address for //2 (Node2). Depends on the infrastructure setup.
const NODE_TO_DISABLE_ADDRESS: &str = "127.0.0.1:9945";
const SESSIONS_TO_MEET_BAN_THRESHOLD: SessionCount = 4;

/// Runs a chain, sets up a committee and validators. Sets an incorrect key for one of the
/// validators. Waits for the offending validator to hit the ban threshold of sessions without
/// producing blocks. Verifies that the offending validator has in fact been banned out for the
/// appropriate reason.
pub fn ban_automatic(config: &Config) -> anyhow::Result<()> {
    let (root_connection, reserved_validators, non_reserved_validators) = setup_test(config)?;

    // Check current era validators.
    check_validators(
        &root_connection,
        &reserved_validators,
        &non_reserved_validators,
        get_current_era_validators,
    );

    check_ban_config(
        &root_connection,
        DEFAULT_BAN_MINIMAL_EXPECTED_PERFORMANCE,
        DEFAULT_BAN_SESSION_COUNT_THRESHOLD,
        DEFAULT_CLEAN_SESSION_COUNTER_DELAY,
    );

    let validator_to_disable =
        &non_reserved_validators[VALIDATOR_TO_DISABLE_NON_RESERVED_INDEX as usize];

    info!(target: "aleph-client", "Validator to disable: {}", validator_to_disable);

    check_underperformed_validator_session_count(&root_connection, validator_to_disable, &0);
    check_underperformed_validator_reason(&root_connection, validator_to_disable, None);

    let validator_seed = get_validator_seed(VALIDATOR_TO_DISABLE_OVERALL_INDEX);
    let stash_controller = NodeKeys::from(validator_seed);
    let controller_key_to_disable = stash_controller.controller;

    // This connection has to be set up with the controller key.
    let connection_to_disable =
        SignedConnection::new(NODE_TO_DISABLE_ADDRESS, controller_key_to_disable);

    set_invalid_keys_for_validator(&connection_to_disable)?;

    let current_session = get_current_session(&root_connection);

    wait_for_at_least_session(
        &root_connection,
        current_session + SESSIONS_TO_MEET_BAN_THRESHOLD,
    )?;

    // The session count for underperforming validators is reset to 0 immediately on reaching the
    // threshold.
    check_underperformed_validator_session_count(&root_connection, validator_to_disable, &0);

    let reason = BanReason::InsufficientUptime(DEFAULT_BAN_SESSION_COUNT_THRESHOLD);
    let start = get_current_era(&root_connection) + 1;
    let expected_ban_info = BanInfo { reason, start };

    check_underperformed_validator_reason(
        &root_connection,
        validator_to_disable,
        Some(&expected_ban_info),
    );

    let expected_non_reserved =
        &non_reserved_validators[(VALIDATOR_TO_DISABLE_NON_RESERVED_INDEX + 1) as usize..];

    let expected_banned_validators = vec![(validator_to_disable.clone(), expected_ban_info)];
    check_ban_event(&root_connection, &expected_banned_validators)?;

    // Check current validators.
    check_validators(
        &root_connection,
        &reserved_validators,
        expected_non_reserved,
        get_current_era_validators,
    );

    Ok(())
}
