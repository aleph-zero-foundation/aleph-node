use aleph_client::{
    change_validators, get_ban_config, get_ban_info_for_validator, get_block_hash,
    get_session_period, get_underperformed_validator_session_count, wait_for_event,
    wait_for_full_era_completion, AccountId, AnyConnection, RootConnection, XtStatus,
};
use codec::Decode;
use log::info;
use primitives::{BanConfig, BanInfo, CommitteeSeats, EraValidators, SessionCount, SessionIndex};
use sp_core::H256;
use sp_runtime::Perbill;

use crate::{
    accounts::account_ids_from_keys, elections::get_members_subset_for_session,
    validators::get_test_validators, Config,
};

const RESERVED_SEATS: u32 = 2;
const NON_RESERVED_SEATS: u32 = 2;

type BanTestSetup = (
    RootConnection,
    Vec<AccountId>,
    Vec<AccountId>,
    CommitteeSeats,
);

pub fn setup_test(config: &Config) -> anyhow::Result<BanTestSetup> {
    let root_connection = config.create_root_connection();

    let validator_keys = get_test_validators(config);
    let reserved_validators = account_ids_from_keys(&validator_keys.reserved);
    let non_reserved_validators = account_ids_from_keys(&validator_keys.non_reserved);

    let seats = CommitteeSeats {
        reserved_seats: RESERVED_SEATS,
        non_reserved_seats: NON_RESERVED_SEATS,
    };

    change_validators(
        &root_connection,
        Some(reserved_validators.clone()),
        Some(non_reserved_validators.clone()),
        Some(seats),
        XtStatus::InBlock,
    );

    wait_for_full_era_completion(&root_connection)?;

    Ok((
        root_connection,
        reserved_validators,
        non_reserved_validators,
        seats,
    ))
}

pub fn check_validators<C: AnyConnection>(
    connection: &C,
    expected_reserved: &[AccountId],
    expected_non_reserved: &[AccountId],
    actual_validators_source: fn(&C) -> EraValidators<AccountId>,
) -> EraValidators<AccountId> {
    let era_validators = actual_validators_source(connection);

    assert_eq!(era_validators.reserved, expected_reserved);
    assert_eq!(era_validators.non_reserved, expected_non_reserved);

    era_validators
}

pub fn check_ban_config(
    connection: &RootConnection,
    expected_minimal_expected_performance: Perbill,
    expected_session_count_threshold: SessionCount,
    expected_clean_session_counter_delay: SessionCount,
) -> BanConfig {
    let ban_config = get_ban_config(connection);

    assert_eq!(
        ban_config.minimal_expected_performance,
        expected_minimal_expected_performance
    );
    assert_eq!(
        ban_config.underperformed_session_count_threshold,
        expected_session_count_threshold
    );
    assert_eq!(
        ban_config.clean_session_counter_delay,
        expected_clean_session_counter_delay
    );

    ban_config
}

pub fn check_underperformed_validator_session_count<C: AnyConnection>(
    connection: &C,
    validator: &AccountId,
    expected_session_count: &SessionCount,
    block_hash: Option<H256>,
) -> SessionCount {
    let underperformed_validator_session_count =
        get_underperformed_validator_session_count(connection, validator, block_hash);

    assert_eq!(
        &underperformed_validator_session_count,
        expected_session_count
    );

    underperformed_validator_session_count
}

pub fn check_ban_info_for_validator<C: AnyConnection>(
    connection: &C,
    validator: &AccountId,
    expected_info: Option<&BanInfo>,
) -> Option<BanInfo> {
    let validator_ban_info = get_ban_info_for_validator(connection, validator);

    assert_eq!(validator_ban_info.as_ref(), expected_info);

    validator_ban_info
}

#[derive(Debug, Decode, Clone)]
pub struct BanEvent {
    banned_validators: Vec<(AccountId, BanInfo)>,
}

pub fn check_ban_event<C: AnyConnection>(
    connection: &C,
    expected_banned_validators: &[(AccountId, BanInfo)],
) -> anyhow::Result<BanEvent> {
    let event = wait_for_event(connection, ("Elections", "BanValidators"), |e: BanEvent| {
        info!("Received BanValidators event: {:?}", e.banned_validators);
        assert_eq!(e.banned_validators, expected_banned_validators);
        true
    })?;

    Ok(event)
}

pub fn get_members_for_session(
    reserved_validators: &[AccountId],
    non_reserved_validators: &[AccountId],
    seats: &CommitteeSeats,
    session: SessionIndex,
) -> Vec<AccountId> {
    let reserved_members =
        get_members_subset_for_session(seats.reserved_seats, reserved_validators, session);
    let non_reserved_members =
        get_members_subset_for_session(seats.non_reserved_seats, non_reserved_validators, session);
    reserved_members
        .into_iter()
        .chain(non_reserved_members.into_iter())
        .collect()
}

/// Checks whether underperformed counts for validators change predictably. Assumes: (a) that the
/// sessions checked are in the past, (b) that all the checked validators are underperforming in
/// those sessions (e.g. due to a prohibitively high performance threshold).
pub fn check_underperformed_count_for_sessions<C: AnyConnection>(
    connection: &C,
    reserved_validators: &[AccountId],
    non_reserved_validators: &[AccountId],
    seats: &CommitteeSeats,
    start_session: SessionIndex,
    end_session: SessionIndex,
    ban_session_threshold: SessionCount,
) -> anyhow::Result<()> {
    let session_period = get_session_period(connection);

    let validators: Vec<_> = reserved_validators
        .iter()
        .chain(non_reserved_validators.iter())
        .collect();

    for session in start_session..end_session {
        let session_end_block = (session + 1) * session_period;
        let session_end_block_hash = get_block_hash(connection, session_end_block);

        let previous_session_end_block = session_end_block - session_period;
        let previous_session_end_block_hash =
            get_block_hash(connection, previous_session_end_block);

        let members =
            get_members_for_session(reserved_validators, non_reserved_validators, seats, session);

        validators.iter().for_each(|&val| {
            info!(
                "Checking session count | session {} | validator {}",
                session, val
            );
            let session_underperformed_count = get_underperformed_validator_session_count(
                connection,
                val,
                Some(session_end_block_hash),
            );
            let previous_session_underperformed_count = get_underperformed_validator_session_count(
                connection,
                val,
                Some(previous_session_end_block_hash),
            );

            let underperformed_diff =
                session_underperformed_count.abs_diff(previous_session_underperformed_count);

            if members.contains(val) {
                // Counter for committee members legally incremented by 1 or reset to 0 (decremented
                // by ban_session_threshold - 1).
                if underperformed_diff != 1 && underperformed_diff != (ban_session_threshold - 1) {
                    panic!(
                        "Underperformed session count for committee validator {} for session {} changed from {} to {}.",
                        val, session, previous_session_underperformed_count, session_underperformed_count
                    );
                }
            } else if underperformed_diff != 0 {
                // Counter for validators on the bench should stay the same.
                panic!(
                    "Underperformed session count for non-committee validator {} for session {} changed from {} to {}.",
                    val, session, previous_session_underperformed_count, session_underperformed_count
                );
            }
        });
    }

    Ok(())
}
