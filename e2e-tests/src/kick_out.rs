use aleph_client::{
    change_validators, get_committee_kick_out_config, get_kick_out_reason_for_validator,
    get_underperformed_validator_session_count, wait_for_event, wait_for_full_era_completion,
    AccountId, AnyConnection, RootConnection, XtStatus,
};
use codec::Decode;
use log::info;
use primitives::{
    CommitteeKickOutConfig, CommitteeSeats, EraValidators, KickOutReason, SessionCount,
};
use sp_runtime::Perbill;

use crate::{accounts::account_ids_from_keys, validators::get_test_validators, Config};

pub fn setup_test(
    config: &Config,
) -> anyhow::Result<(RootConnection, Vec<AccountId>, Vec<AccountId>)> {
    let root_connection = config.create_root_connection();

    let validator_keys = get_test_validators(config);
    let reserved_validators = account_ids_from_keys(&validator_keys.reserved);
    let non_reserved_validators = account_ids_from_keys(&validator_keys.non_reserved);

    let seats = CommitteeSeats {
        reserved_seats: 2,
        non_reserved_seats: 2,
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

pub fn check_committee_kick_out_config(
    connection: &RootConnection,
    expected_minimal_expected_performance: Perbill,
    expected_session_count_threshold: SessionCount,
    expected_clean_session_counter_delay: SessionCount,
) -> CommitteeKickOutConfig {
    let committee_kick_out_config = get_committee_kick_out_config(connection);

    assert_eq!(
        committee_kick_out_config.minimal_expected_performance,
        expected_minimal_expected_performance
    );
    assert_eq!(
        committee_kick_out_config.underperformed_session_count_threshold,
        expected_session_count_threshold
    );
    assert_eq!(
        committee_kick_out_config.clean_session_counter_delay,
        expected_clean_session_counter_delay
    );

    committee_kick_out_config
}

pub fn check_underperformed_validator_session_count<C: AnyConnection>(
    connection: &C,
    validator: &AccountId,
    expected_session_count: &SessionCount,
) -> SessionCount {
    let underperformed_validator_session_count =
        get_underperformed_validator_session_count(connection, validator);

    assert_eq!(
        &underperformed_validator_session_count,
        expected_session_count
    );

    underperformed_validator_session_count
}

pub fn check_underperformed_validator_reason<C: AnyConnection>(
    connection: &C,
    validator: &AccountId,
    expected_reason: Option<&KickOutReason>,
) -> Option<KickOutReason> {
    let validator_kick_out_reason = get_kick_out_reason_for_validator(connection, validator);

    assert_eq!(validator_kick_out_reason.as_ref(), expected_reason);

    validator_kick_out_reason
}

#[derive(Debug, Decode, Clone)]
pub struct KickOutEvent {
    kicked_out_validators: Vec<(AccountId, KickOutReason)>,
}

pub fn check_kick_out_event<C: AnyConnection>(
    connection: &C,
    expected_kicked_out_validators: &[(AccountId, KickOutReason)],
) -> anyhow::Result<KickOutEvent> {
    let event = wait_for_event(
        connection,
        ("Elections", "KickOutValidators"),
        |e: KickOutEvent| {
            info!(
                "Received KickOutValidators event: {:?}",
                e.kicked_out_validators
            );
            assert_eq!(e.kicked_out_validators, expected_kicked_out_validators);
            true
        },
    )?;

    Ok(event)
}
