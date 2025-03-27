use std::collections::HashSet;

use aleph_client::{
    pallets::{
        aleph::{AlephApi, AlephSudoApi},
        committee_management::CommitteeManagementApi,
        elections::{ElectionsApi, ElectionsSudoApi},
        session::SessionApi,
    },
    primitives::{CommitteeSeats, EraValidators},
    utility::BlocksApi,
    waiting::{AlephWaiting, BlockStatus, WaitingExt},
    AccountId, RootConnection, SignedConnection, TxStatus,
};
use aleph_client::pallets::committee_management::CommitteeManagementSudoApi;
use log::info;
use primitives::{
    SessionCount, DEFAULT_FINALITY_BAN_MINIMAL_EXPECTED_PERFORMANCE,
    DEFAULT_FINALITY_BAN_SESSION_COUNT_THRESHOLD,
};

use crate::{
    accounts::{account_ids_from_keys, get_validator_seed, NodeKeys},
    config::Config,
    rewards::set_invalid_keys_for_validator,
    validators::get_test_validators,
};

// all below consts are related to each other, and they correspond to local chain setup:
// there are exactly 4 validator nodes run locally, //1 and //2 are reserved and //3 and //4 are non-reserved
// we're going to disable in one of the tests exactly validator with seed //3, which has RPC address port 9948
const RESERVED_SEATS: u32 = 2;
const NON_RESERVED_SEATS: u32 = 2;
// since we keep non-reserved account ids in a separate array, node //3 is the first account on that list
const NON_RESERVED_DEAD_INDEX: usize = 0;
const NODE_TO_DISABLE_ADDRESS: &str = "ws://127.0.0.1:9948";
const VALIDATOR_TO_DISABLE_OVERALL_INDEX: u32 = 3;

// version which is required for scores to be enabled
const ABFT_PERFORMANCE_VERSION: u32 = 5;

#[tokio::test]
async fn all_validators_have_ideal_performance() -> anyhow::Result<()> {
    let config = crate::config::setup_test();
    let (root_connection, reserved_validators, non_reserved_validators, _) =
        setup_test(config).await?;
    let all_validators = reserved_validators
        .iter()
        .chain(non_reserved_validators.iter());

    set_finality_version(ABFT_PERFORMANCE_VERSION, &root_connection).await?;

    check_validators(
        &reserved_validators,
        &non_reserved_validators,
        root_connection.get_current_era_validators(None).await,
    );

    check_finality_ban_config(
        &root_connection,
        DEFAULT_FINALITY_BAN_MINIMAL_EXPECTED_PERFORMANCE,
        DEFAULT_FINALITY_BAN_SESSION_COUNT_THRESHOLD,
    )
    .await;

    for validator in all_validators.clone() {
        check_underperformed_validator_session_count(&root_connection, validator, 0).await
    }

    let session_id = root_connection.get_session(None).await;

    root_connection
        .wait_for_n_sessions(1, BlockStatus::Best)
        .await;

    let block = root_connection.last_block_of_session(session_id).await?;
    let scores = root_connection
        .abft_scores(session_id, block)
        .await
        .unwrap();
    assert!(scores.points.into_iter().all(|point| point <= 4));

    for validator in all_validators {
        check_underperformed_validator_session_count(&root_connection, validator, 0).await
    }
    Ok(())
}

#[tokio::test]
async fn one_validator_is_dead() -> anyhow::Result<()> {
    let config = crate::config::setup_test();
    let (root_connection, reserved_validators, non_reserved_validators, _) =
        setup_test(config).await?;

    set_finality_version(ABFT_PERFORMANCE_VERSION, &root_connection).await?;

    check_validators(
        &reserved_validators,
        &non_reserved_validators,
        root_connection.get_current_era_validators(None).await,
    );

    let production_underperformed_threshold = 9;
    info!("Increasing production ban config threshold to {} sessions", production_underperformed_threshold);
    root_connection
        .set_ban_config(None, Some(production_underperformed_threshold), None, None, TxStatus::InBlock)
        .await?;
    let ban_config = root_connection.get_ban_config(None).await;
    assert_eq!(
        ban_config.underperformed_session_count_threshold,
        production_underperformed_threshold
    );
    check_finality_ban_config(
        &root_connection,
        DEFAULT_FINALITY_BAN_MINIMAL_EXPECTED_PERFORMANCE,
        DEFAULT_FINALITY_BAN_SESSION_COUNT_THRESHOLD,
    )
    .await;

    let validator_to_disable = &non_reserved_validators[NON_RESERVED_DEAD_INDEX];
    info!("Validator to disable: {}", validator_to_disable);
    check_underperformed_validator_session_count(&root_connection, validator_to_disable, 0).await;
    disable_validator( NODE_TO_DISABLE_ADDRESS, VALIDATOR_TO_DISABLE_OVERALL_INDEX).await?;

    // Validator has been disabled, let's wait one session in which it's disabled.
    let score_session_id = root_connection.get_session(None).await + 1;
    root_connection
        .wait_for_session(score_session_id + 1, BlockStatus::Best)
        .await;

    let scores = root_connection
        .abft_scores(score_session_id, None)
        .await
        .unwrap();

    assert!(scores.points.iter().any(|p| *p >= 100), "Points {:?}", scores.points);

    Ok(())
}

async fn set_finality_version(finality_version: u32, root_connection: &RootConnection) -> anyhow::Result<()> {
    let current_finality_version = root_connection.finality_version(None).await;
    if current_finality_version < ABFT_PERFORMANCE_VERSION {
        change_finality_version(finality_version, root_connection).await?
    }

    // In this session first performance metrics are sent, we have to wait some time
    // to make sure that we don't check storage before first score is sent.
    root_connection
        .wait_for_n_sessions(1, BlockStatus::Best)
        .await;
    Ok(())
}

async fn setup_test(
    config: &Config,
) -> anyhow::Result<(
    RootConnection,
    Vec<AccountId>,
    Vec<AccountId>,
    CommitteeSeats,
)> {
    let root_connection = config.create_root_connection().await;

    let validator_keys = get_test_validators(config);
    let reserved_validators = account_ids_from_keys(&validator_keys.reserved);
    let non_reserved_validators = account_ids_from_keys(&validator_keys.non_reserved).into_iter().take(2).collect::<Vec<_>>();
    let seats = CommitteeSeats {
        reserved_seats: RESERVED_SEATS,
        non_reserved_seats: NON_RESERVED_SEATS,
        non_reserved_finality_seats: NON_RESERVED_SEATS,
    };

    let current_era_valdators = root_connection.get_current_era_validators(None).await;
    let current_committee_seats = root_connection.get_committee_seats(None).await;

    let mut send_change_validators_xt = false;
    let mut maybe_new_reserved = None;
    if current_era_valdators.reserved != reserved_validators {
        maybe_new_reserved = Some(reserved_validators.clone());
        send_change_validators_xt = true;
    }

    let mut maybe_new_non_reserved = None;
    if current_era_valdators.non_reserved != non_reserved_validators {
        maybe_new_non_reserved = Some(non_reserved_validators.clone());
        send_change_validators_xt = true;
    }

    let mut maybe_new_committee_seats = None;
    if current_committee_seats != seats {
        maybe_new_committee_seats = Some(seats.clone());
        send_change_validators_xt = true;
    }

    if send_change_validators_xt {
        info!(
            "Changing validators {:?}.",
            (
                &maybe_new_reserved,
                &maybe_new_non_reserved,
                &maybe_new_committee_seats
            )
        );
        root_connection
            .change_validators(
                maybe_new_reserved,
                maybe_new_non_reserved,
                maybe_new_committee_seats,
                TxStatus::InBlock,
            )
            .await?;

        root_connection.wait_for_n_eras(1, BlockStatus::Best).await;
        info!("Validators are changed.");
    }

    Ok((
        root_connection,
        reserved_validators,
        non_reserved_validators,
        seats,
    ))
}

fn check_validators(
    expected_reserved: &[AccountId],
    expected_non_reserved: &[AccountId],
    era_validators: EraValidators<AccountId>,
) -> EraValidators<AccountId> {
    assert_eq!(
        HashSet::<_>::from_iter(&era_validators.reserved),
        HashSet::<_>::from_iter(expected_reserved)
    );
    assert_eq!(
        HashSet::<_>::from_iter(&era_validators.non_reserved),
        HashSet::<_>::from_iter(expected_non_reserved)
    );

    era_validators
}

async fn change_finality_version<C: SessionApi + AlephSudoApi + AlephWaiting>(
    finality_version: u32,
    connection: &C,
) -> anyhow::Result<()> {
    info!("Changing finality version to 5.");
    let session_for_upgrade = connection.get_session(None).await + 2;
    connection
        .schedule_finality_version_change(
            finality_version,
            session_for_upgrade,
            TxStatus::InBlock,
        )
        .await?;
    connection
        .wait_for_session(session_for_upgrade, BlockStatus::Best)
        .await;
    info!("Finality version is changed.");

    Ok(())
}

async fn check_finality_ban_config<C: CommitteeManagementApi>(
    connection: &C,
    expected_minimal_expected_performance: u16,
    expected_session_count_threshold: SessionCount,
) {
    let ban_config = connection.get_finality_ban_config(None).await;

    assert_eq!(
        ban_config.minimal_expected_performance,
        expected_minimal_expected_performance
    );
    assert_eq!(
        ban_config.underperformed_session_count_threshold,
        expected_session_count_threshold
    );
}

async fn check_underperformed_validator_session_count<C: CommitteeManagementApi>(
    connection: &C,
    validator: &AccountId,
    expected_session_count: SessionCount,
) {
    let underperformed_validator_session_count = connection
        .get_underperformed_finalizer_session_count(validator.clone(), None)
        .await
        .unwrap_or_default();

    assert_eq!(
        underperformed_validator_session_count, expected_session_count,
        "{}",
        validator
    );
}

async fn disable_validator(validator_address: &str, validator_seed: u32) -> anyhow::Result<()> {
    let validator_seed = get_validator_seed(validator_seed);
    let stash_controller = NodeKeys::from(validator_seed).validator;

    // This connection has to be set up with the controller key.
    let connection_to_disable = SignedConnection::new(validator_address, stash_controller).await;

    set_invalid_keys_for_validator(vec![connection_to_disable]).await
}
