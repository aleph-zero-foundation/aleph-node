use aleph_client::{
    get_current_era, get_current_session, staking_force_new_era, wait_for_full_era_completion,
    wait_for_next_era, wait_for_session, AccountId, SignedConnection, XtStatus,
};
use log::info;
use primitives::{
    staking::MIN_VALIDATOR_BOND, CommitteeSeats, EraIndex, EraValidators, SessionIndex,
};

use crate::{
    elections::get_and_test_members_for_session,
    rewards::{
        check_points, get_era_for_session, reset_validator_keys, set_invalid_keys_for_validator,
        setup_validators, validators_bond_extra_stakes,
    },
    Config,
};

// Maximum difference between fractions of total reward that a validator gets.
// Two values are compared: one calculated in tests and the other one based on data
// retrieved from pallet Staking.
const MAX_DIFFERENCE: f64 = 0.07;

pub fn points_basic(config: &Config) -> anyhow::Result<()> {
    let (era_validators, committee_size, start_session) = setup_validators(config)?;

    let connection = config.get_first_signed_connection();

    wait_for_next_era(&connection)?;
    let end_session = get_current_session(&connection);
    let members_per_session = committee_size.reserved_seats + committee_size.non_reserved_seats;

    info!(
        "Checking rewards for sessions {}..{}.",
        start_session, end_session
    );

    for session in start_session..end_session {
        let era = get_era_for_session(&connection, session);
        let (members_active, members_bench) =
            get_and_test_members_for_session(&connection, committee_size, &era_validators, session);

        check_points(
            &connection,
            session,
            era,
            members_active,
            members_bench,
            members_per_session,
            MAX_DIFFERENCE,
        )?
    }

    Ok(())
}

/// Runs a chain, bonds extra stakes to validator accounts and checks that reward points
/// are calculated correctly afterward.
pub fn points_stake_change(config: &Config) -> anyhow::Result<()> {
    let (era_validators, committee_size, _) = setup_validators(config)?;

    validators_bond_extra_stakes(
        config,
        &[
            8 * MIN_VALIDATOR_BOND,
            6 * MIN_VALIDATOR_BOND,
            4 * MIN_VALIDATOR_BOND,
            2 * MIN_VALIDATOR_BOND,
            0,
        ],
    );

    let connection = config.get_first_signed_connection();
    let start_session = get_current_session(&connection);
    wait_for_next_era(&connection)?;
    let end_session = get_current_session(&connection);
    let members_per_session = committee_size.reserved_seats + committee_size.non_reserved_seats;

    info!(
        "Checking rewards for sessions {}..{}.",
        start_session, end_session
    );

    for session in start_session..end_session {
        let era = get_era_for_session(&connection, session);
        let (members_active, members_bench) =
            get_and_test_members_for_session(&connection, committee_size, &era_validators, session);

        check_points(
            &connection,
            session,
            era,
            members_active,
            members_bench,
            members_per_session,
            MAX_DIFFERENCE,
        )?
    }

    Ok(())
}

/// Runs a chain, sets invalid session keys for one validator, re-sets the keys to valid ones
/// and checks that reward points are calculated correctly afterward.
pub fn disable_node(config: &Config) -> anyhow::Result<()> {
    let (era_validators, committee_size, start_session) = setup_validators(config)?;

    let root_connection = config.create_root_connection();
    let controller_connection = SignedConnection::new(&config.node, config.node_keys().controller);

    // this should `disable` this node by setting invalid session_keys
    set_invalid_keys_for_validator(&controller_connection)?;
    // this should `re-enable` this node, i.e. by means of the `rotate keys` procedure
    reset_validator_keys(&controller_connection)?;

    wait_for_full_era_completion(&root_connection)?;
    let end_session = get_current_session(&root_connection);
    let members_per_session = committee_size.reserved_seats + committee_size.non_reserved_seats;

    info!(
        "Checking rewards for sessions {}..{}.",
        start_session, end_session
    );

    for session in start_session..end_session {
        let era = get_era_for_session(&controller_connection, session);
        let (members_active, members_bench) = get_and_test_members_for_session(
            &controller_connection,
            committee_size,
            &era_validators,
            session,
        );

        check_points(
            &controller_connection,
            session,
            era,
            members_active,
            members_bench,
            members_per_session,
            MAX_DIFFERENCE,
        )?;
    }

    Ok(())
}

/// Runs a chain, forces a new era to begin, checks that reward points are calculated correctly
/// for 3 sessions: 1) immediately following the forcing call, 2) in the subsequent, interim
/// session, when the new era has not yet started, 3) in the next session, second one after
/// the call, when the new era has already begun.
pub fn force_new_era(config: &Config) -> anyhow::Result<()> {
    let (era_validators, committee_size, start_session) = setup_validators(config)?;

    let connection = config.get_first_signed_connection();
    let root_connection = config.create_root_connection();
    let start_era = get_era_for_session(&connection, start_session);

    info!("Start | era: {}, session: {}", start_era, start_session);

    staking_force_new_era(&root_connection, XtStatus::Finalized);

    wait_for_session(&connection, start_session + 2)?;
    let current_era = get_current_era(&connection);
    let current_session = get_current_session(&connection);
    info!(
        "After ForceNewEra | era: {}, session: {}",
        current_era, current_session
    );

    check_points_after_force_new_era(
        &connection,
        start_session,
        start_era,
        &era_validators,
        committee_size,
        MAX_DIFFERENCE,
    )?;
    Ok(())
}

/// Change stake and force new era: checks if reward points are calculated properly
/// in a scenario in which stakes are changed for each validator, and then a new era is forced.
///
/// Expected behaviour: until the next (forced) era, rewards are calculated using old stakes,
/// and after two sessions (required for a new era to be forced) they are adjusted to the new
/// stakes.
pub fn change_stake_and_force_new_era(config: &Config) -> anyhow::Result<()> {
    let (era_validators, committee_size, start_session) = setup_validators(config)?;

    let connection = config.get_first_signed_connection();
    let root_connection = config.create_root_connection();

    let start_era = get_era_for_session(&connection, start_session);
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
    );

    staking_force_new_era(&root_connection, XtStatus::Finalized);

    wait_for_session(&connection, start_session + 2)?;
    let current_era = get_current_era(&connection);
    let current_session = get_current_session(&connection);
    info!(
        "After ForceNewEra | era: {}, session: {}",
        current_era, current_session
    );

    check_points_after_force_new_era(
        &connection,
        start_session,
        start_era,
        &era_validators,
        committee_size,
        MAX_DIFFERENCE,
    )?;
    Ok(())
}

fn check_points_after_force_new_era(
    connection: &SignedConnection,
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

        let (members_active, members_bench) =
            get_and_test_members_for_session(connection, seats, era_validators, session_to_check);

        check_points(
            connection,
            session_to_check,
            era_to_check,
            members_active,
            members_bench,
            seats.reserved_seats + seats.non_reserved_seats,
            max_relative_difference,
        )?;
    }
    Ok(())
}
