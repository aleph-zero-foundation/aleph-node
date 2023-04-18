use aleph_client::{
    pallets::{
        committee_management::CommitteeManagementApi,
        elections::ElectionsApi,
        session::SessionApi,
        staking::{StakingApi, StakingSudoApi},
    },
    primitives::{CommitteeSeats, EraValidators},
    utility::{BlocksApi, SessionEraApi},
    waiting::{AlephWaiting, BlockStatus, WaitingExt},
    AccountId, SignedConnection, SignedConnectionApi, TxStatus,
};
use log::info;
use primitives::{staking::MIN_VALIDATOR_BOND, EraIndex, SessionIndex};

use crate::{
    config::setup_test,
    elections::get_and_test_members_for_session,
    rewards::{
        check_points, reset_validator_keys, set_invalid_keys_for_validator, setup_validators,
        validators_bond_extra_stakes,
    },
};

// Maximum difference between fractions of total reward that a validator gets.
// Two values are compared: one calculated in tests and the other one based on data
// retrieved from pallet Staking.
const MAX_DIFFERENCE: f64 = 0.07;

#[tokio::test]
pub async fn points_basic() -> anyhow::Result<()> {
    let config = setup_test();
    let (era_validators, committee_size, start_session) = setup_validators(config).await?;

    let connection = config.get_first_signed_connection().await;

    connection.wait_for_n_eras(1, BlockStatus::Best).await;
    let end_session = connection.get_session(None).await;
    let members_per_session = committee_size.reserved_seats + committee_size.non_reserved_seats;

    info!(
        "Checking rewards for sessions {}..{}.",
        start_session, end_session
    );

    for session in start_session..end_session {
        let era = connection.get_active_era_for_session(session).await?;
        let (members_active, members_bench) = get_and_test_members_for_session(
            &connection,
            committee_size.clone(),
            &era_validators,
            session,
        )
        .await?;

        check_points(
            &connection,
            session,
            era,
            members_active,
            members_bench,
            members_per_session,
            MAX_DIFFERENCE,
        )
        .await?
    }

    Ok(())
}

/// Runs a chain, bonds extra stakes to validator accounts and checks that reward points
/// are calculated correctly afterward.
#[tokio::test]
pub async fn points_stake_change() -> anyhow::Result<()> {
    let config = setup_test();
    let (era_validators, committee_size, _) = setup_validators(config).await?;

    validators_bond_extra_stakes(
        config,
        &[
            8 * MIN_VALIDATOR_BOND,
            6 * MIN_VALIDATOR_BOND,
            4 * MIN_VALIDATOR_BOND,
            2 * MIN_VALIDATOR_BOND,
            0,
        ],
    )
    .await;

    let connection = config.get_first_signed_connection().await;
    let start_session = connection.get_session(None).await;
    connection.wait_for_n_eras(1, BlockStatus::Best).await;
    let end_session = connection.get_session(None).await;
    let members_per_session = committee_size.reserved_seats + committee_size.non_reserved_seats;

    info!(
        "Checking rewards for sessions {}..{}.",
        start_session, end_session
    );

    for session in start_session..end_session {
        let era = connection.get_active_era_for_session(session).await?;
        let (members_active, members_bench) = get_and_test_members_for_session(
            &connection,
            committee_size.clone(),
            &era_validators,
            session,
        )
        .await?;

        check_points(
            &connection,
            session,
            era,
            members_active,
            members_bench,
            members_per_session,
            MAX_DIFFERENCE,
        )
        .await?
    }

    Ok(())
}

/// Runs a chain, sets invalid session keys for one validator, re-sets the keys to valid ones
/// and checks that reward points are calculated correctly afterward.
#[tokio::test]
pub async fn disable_node() -> anyhow::Result<()> {
    let config = setup_test();
    let (era_validators, committee_size, start_session) = setup_validators(config).await?;

    let root_connection = config.create_root_connection().await;
    let controller_connection =
        SignedConnection::new(&config.node, config.node_keys().controller).await;

    // this should `disable` this node by setting invalid session_keys
    set_invalid_keys_for_validator(vec![controller_connection.clone()]).await?;
    // this should `re-enable` this node, i.e. by means of the `rotate keys` procedure
    reset_validator_keys(&controller_connection).await?;

    root_connection.wait_for_n_eras(1, BlockStatus::Best).await;
    let end_session = root_connection.get_session(None).await;
    let members_per_session = committee_size.reserved_seats + committee_size.non_reserved_seats;

    info!(
        "Checking rewards for sessions {}..{}.",
        start_session, end_session
    );

    for session in start_session..end_session {
        let era = root_connection.get_active_era_for_session(session).await?;
        let (members_active, members_bench) = get_and_test_members_for_session(
            &controller_connection,
            committee_size.clone(),
            &era_validators,
            session,
        )
        .await?;

        check_points(
            &controller_connection,
            session,
            era,
            members_active,
            members_bench,
            members_per_session,
            MAX_DIFFERENCE,
        )
        .await?;
    }

    Ok(())
}

/// Runs a chain, forces a new era to begin, checks that reward points are calculated correctly
/// for 3 sessions: 1) immediately following the forcing call, 2) in the subsequent, interim
/// session, when the new era has not yet started, 3) in the next session, second one after
/// the call, when the new era has already begun.
#[tokio::test]
pub async fn force_new_era() -> anyhow::Result<()> {
    let config = setup_test();
    let (era_validators, committee_size, start_session) = setup_validators(config).await?;

    let connection = config.get_first_signed_connection().await;
    let root_connection = config.create_root_connection().await;
    let start_era = connection.get_active_era_for_session(start_session).await?;

    info!("Start | era: {}, session: {}", start_era, start_session);

    root_connection.force_new_era(TxStatus::Finalized).await?;
    connection
        .wait_for_session(start_session + 2, BlockStatus::Finalized)
        .await;
    let active_era = connection.get_active_era(None).await;
    let current_session = connection.get_session(None).await;
    info!(
        "After ForceNewEra | era: {}, session: {}",
        active_era, current_session
    );

    check_points_after_force_new_era(
        &connection,
        start_session,
        start_era,
        &era_validators,
        committee_size,
        MAX_DIFFERENCE,
    )
    .await?;
    Ok(())
}

/// Change stake and force new era: checks if reward points are calculated properly
/// in a scenario in which stakes are changed for each validator, and then a new era is forced.
///
/// Expected behaviour: until the next (forced) era, rewards are calculated using old stakes,
/// and after two sessions (required for a new era to be forced) they are adjusted to the new
/// stakes.
#[tokio::test]
pub async fn change_stake_and_force_new_era() -> anyhow::Result<()> {
    let config = setup_test();
    let (era_validators, committee_size, start_session) = setup_validators(config).await?;

    let connection = config.get_first_signed_connection().await;
    let root_connection = config.create_root_connection().await;

    let start_era = connection.get_active_era_for_session(start_session).await?;
    info!("Start | era: {}, session: {}", start_era, start_session);

    validators_bond_extra_stakes(
        config,
        &[
            7 * MIN_VALIDATOR_BOND,
            2 * MIN_VALIDATOR_BOND,
            11 * MIN_VALIDATOR_BOND,
            0,
            4 * MIN_VALIDATOR_BOND,
        ],
    )
    .await;

    root_connection.force_new_era(TxStatus::Finalized).await?;
    let start_session = root_connection.get_session(None).await;
    connection
        .wait_for_session(start_session + 2, BlockStatus::Finalized)
        .await;
    let active_era = connection.get_active_era(None).await;
    let current_session = connection.get_session(None).await;
    info!(
        "After ForceNewEra | era: {}, session: {}",
        active_era, current_session
    );

    check_points_after_force_new_era(
        &connection,
        start_session,
        start_era,
        &era_validators,
        committee_size,
        MAX_DIFFERENCE,
    )
    .await?;
    Ok(())
}

async fn check_points_after_force_new_era<
    S: SignedConnectionApi
        + BlocksApi
        + ElectionsApi
        + CommitteeManagementApi
        + AlephWaiting
        + StakingApi,
>(
    connection: &S,
    start_session: SessionIndex,
    start_era: EraIndex,
    era_validators: &EraValidators<AccountId>,
    seats: CommitteeSeats,
    max_relative_difference: f64,
) -> anyhow::Result<()> {
    // Once a new era is forced in session k, the new era does not come into effect until session
    // k + 2; we test points:
    // 1) immediately following the call in session k,
    // 2) in the interim session k + 1,
    // 3) in session k + 2, the first session of the new era.
    for idx in 0..3 {
        let session_to_check = start_session + idx;
        let era_to_check = start_era + idx / 2;

        info!(
            "Testing points | era: {}, session: {}",
            era_to_check, session_to_check
        );

        let (members_active, members_bench) = get_and_test_members_for_session(
            connection,
            seats.clone(),
            era_validators,
            session_to_check,
        )
        .await?;

        check_points(
            connection,
            session_to_check,
            era_to_check,
            members_active,
            members_bench,
            seats.reserved_seats + seats.non_reserved_seats,
            max_relative_difference,
        )
        .await?;
    }
    Ok(())
}
