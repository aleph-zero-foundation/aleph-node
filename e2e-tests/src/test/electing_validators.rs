use std::collections::BTreeSet;

use aleph_client::{
    pallets::{
        elections::ElectionsSudoApi,
        session::SessionApi,
        staking::{StakingApi, StakingRawApi, StakingUserApi},
    },
    primitives::CommitteeSeats,
    waiting::{BlockStatus, WaitingExt},
    AccountId, ConnectionApi, KeyPair, Pair, SignedConnection, TxStatus,
};
use log::info;
use primitives::EraIndex;

use crate::{
    config::setup_test,
    validators::{prepare_validators, setup_accounts},
};

/// Verify that `pallet_staking::ErasStakers` contains all target validators.
///
/// We have to do it by comparing keys in storage trie.
async fn assert_validators_are_elected_stakers<C: StakingRawApi>(
    connection: &C,
    current_era: EraIndex,
    expected_validators_as_keys: Vec<Vec<u8>>,
) -> anyhow::Result<()> {
    let stakers = connection
        .get_stakers_storage_keys(current_era, None)
        .await?
        .into_iter()
        .map(|key| key.0);
    let stakers_tree = BTreeSet::from_iter(stakers);
    let expected_validators_as_keys = BTreeSet::from_iter(expected_validators_as_keys);

    assert_eq!(
        expected_validators_as_keys, stakers_tree,
        "Expected another set of staking validators.\n\tExpected: {:?}\n\tActual: {:?}",
        expected_validators_as_keys, stakers_tree
    );

    Ok(())
}

// There are v non-reserved validators and s non-reserved seats. We will have seen all
// the non-reserved validators after ceil(v / s).
fn min_num_sessions_to_see_all_non_reserved_validators(
    non_reserved_count: u32,
    non_reserved_seats: u32,
) -> u32 {
    // Matching done to emphasize handling of `non_reserved_seats` = 0.
    match non_reserved_seats {
        0 => 0,
        _ => {
            // Ceiling without float division.
            (non_reserved_count + non_reserved_seats - 1) / non_reserved_seats
        }
    }
}

/// Verify that all target validators are included `pallet_session::Validators` across a few
/// consecutive sessions.
async fn assert_validators_are_used_as_authorities<C: SessionApi + WaitingExt>(
    connection: &C,
    expected_authorities: &BTreeSet<AccountId>,
    min_num_sessions: u32,
) {
    let mut authorities = BTreeSet::new();

    for _ in 0..min_num_sessions {
        let current_session = connection.get_session(None).await;

        info!("Reading authorities in session {}", current_session);
        let current_authorities = connection.get_validators(None).await;
        for ca in current_authorities.into_iter() {
            authorities.insert(ca);
        }

        connection.wait_for_n_sessions(1, BlockStatus::Best).await;
    }

    assert_eq!(
        *expected_authorities, authorities,
        "Expected another set of authorities.\n\tExpected: {:?}\n\tActual: {:?}",
        expected_authorities, authorities
    );
}

async fn assert_enough_validators<C: ConnectionApi>(connection: &C, min_validator_count: u32) {
    let current_validator_count = connection.get_validators(None).await.len() as u32;
    assert!(
        current_validator_count >= min_validator_count,
        "{} validators present. Staking enforces a minimum of {} validators.",
        current_validator_count,
        min_validator_count
    );
}

fn assert_enough_validators_left_after_chilling(
    reserved_count: u32,
    non_reserved_count: u32,
    reserved_to_chill_count: u32,
    non_reserved_to_chill_count: u32,
    min_validator_count: u32,
) {
    assert!(
        reserved_count >= reserved_to_chill_count,
        "Cannot have less than 0 reserved validators!"
    );
    assert!(
        non_reserved_count >= non_reserved_to_chill_count,
        "Cannot have less than 0 non-reserved validators!"
    );

    let reserved_after_chill_count = reserved_count - reserved_to_chill_count;
    let non_reserved_after_chill_count = non_reserved_count - non_reserved_to_chill_count;
    let validators_after_chill_count = reserved_after_chill_count + non_reserved_after_chill_count;
    assert!(
        validators_after_chill_count >= min_validator_count,
        "{} validators will be left after chilling. Staking enforces a minimum of {} validators.",
        validators_after_chill_count,
        min_validator_count
    );
}

async fn chill_validators(node: &str, chilling: Vec<KeyPair>) {
    for validator in chilling.into_iter() {
        info!("Chilling validator {:?}", validator.signer().public());
        let connection = SignedConnection::new(node, validator).await;
        connection.chill(TxStatus::InBlock).await.unwrap();
    }
}

/// 1. Setup `v` brand new validators (e.g. `v=6`) - `r` reserved (e.g. `r=3`) and `n` (e.g. `n=3`)
/// non reserved.
/// 2. Wait until they are in force.
/// 3. Chill 1 reserved and 1 non-reserved.
/// 4. Verify only staking validators are in force.
///
/// Note:
///  - `pallet_staking` has `MinimumValidatorCount` (usually set to 4 in chain spec) and this cannot be
/// changed on a running chain.
///  - our e2e tests run with 5 validators by default.
/// Thus, running on default settings and chilling 2 validators (1 reserved and 1 non reserved) is
/// a no go: `pallet_staking` will protest and won't proceed with a new committee.
/// To mitigate this, our e2e pipeline accepts a `node-count` parameter to specify the desired
/// number of nodes to run in consensus. Additionally, there is a `min-validator-count`
/// parameter to set `MinimumValidatorCount` in the chain spec as the chain is set up.
/// For this specific test case, we use `node-count = 6` and `min-validator-count = 4`, which
/// satisfies the outlined conditions.
#[tokio::test]
pub async fn authorities_are_staking() -> anyhow::Result<()> {
    let config = setup_test();

    let node = &config.node;
    let root_connection = config.create_root_connection().await;

    const RESERVED_SEATS_DEFAULT: u32 = 3;
    const NON_RESERVED_SEATS_DEFAULT: u32 = 3;

    // `MinimumValidatorCount` from `pallet_staking`, set in chain spec.
    let min_validator_count = root_connection.get_minimum_validator_count(None).await;

    let reserved_seats = match config.test_case_params.reserved_seats {
        Some(seats) => seats,
        None => RESERVED_SEATS_DEFAULT,
    };
    let non_reserved_seats = match config.test_case_params.non_reserved_seats {
        Some(seats) => seats,
        None => NON_RESERVED_SEATS_DEFAULT,
    };

    // Assumes we chill one validator from the reserved and one from the non-reserved pool.
    const RESERVED_TO_CHILL_COUNT: u32 = 1;
    const NON_RESERVED_TO_CHILL_COUNT: u32 = 1;

    assert_enough_validators(&root_connection, min_validator_count).await;

    let desired_validator_count = reserved_seats + non_reserved_seats;
    let accounts = setup_accounts(desired_validator_count);
    prepare_validators(&root_connection, node, &accounts).await?;
    info!("New validators are set up");

    let reserved_validators = accounts.get_stash_accounts()[..reserved_seats as usize].to_vec();
    let chilling_reserved = KeyPair::new(accounts.get_controller_raw_keys()[0].clone()); // first reserved validator
    let non_reserved_validators = accounts.get_stash_accounts()[reserved_seats as usize..].to_vec();
    let chilling_non_reserved =
        KeyPair::new(accounts.get_controller_raw_keys()[reserved_seats as usize].clone()); // first non-reserved validator

    let reserved_count = reserved_validators.len() as u32;
    let non_reserved_count = non_reserved_validators.len() as u32;

    assert_eq!(
        reserved_seats, reserved_count,
        "Desired {} reserved seats, got {}!",
        reserved_seats, reserved_count
    );
    assert_eq!(
        non_reserved_seats, non_reserved_count,
        "Desired {} non-reserved seats, got {}!",
        non_reserved_seats, non_reserved_count
    );

    assert_enough_validators_left_after_chilling(
        reserved_count,
        non_reserved_count,
        RESERVED_TO_CHILL_COUNT,
        NON_RESERVED_TO_CHILL_COUNT,
        min_validator_count,
    );

    root_connection
        .change_validators(
            Some(reserved_validators),
            Some(non_reserved_validators),
            Some(CommitteeSeats {
                reserved_seats,
                non_reserved_seats,
            }),
            TxStatus::Finalized,
        )
        .await?;

    info!("Changed validators to a new set");

    // We need any signed connection.
    let connection = root_connection;
    connection.wait_for_n_eras(2, BlockStatus::Best).await;
    let current_era = connection.get_current_era(None).await;
    info!("New validators are in force (era: {})", current_era);

    assert_validators_are_elected_stakers(
        &connection,
        current_era,
        connection
            .get_stakers_storage_keys_from_accounts(
                current_era,
                accounts.get_stash_accounts(),
                None,
            )
            .await
            .into_iter()
            .map(|k| k.0)
            .collect(),
    )
    .await?;

    let min_num_sessions =
        min_num_sessions_to_see_all_non_reserved_validators(non_reserved_count, non_reserved_seats);

    assert_validators_are_used_as_authorities(
        &connection,
        &BTreeSet::from_iter(accounts.get_stash_accounts().clone().into_iter()),
        min_num_sessions,
    )
    .await;

    chill_validators(node, vec![chilling_reserved, chilling_non_reserved]).await;

    connection.wait_for_n_eras(2, BlockStatus::Best).await;
    let current_era = connection.get_current_era(None).await;
    info!(
        "Subset of validators should be in force (era: {})",
        current_era
    );

    let mut left_stashes = accounts.get_stash_accounts().clone();
    left_stashes.remove(reserved_seats as usize);
    left_stashes.remove(0);

    assert_validators_are_elected_stakers(
        &connection,
        current_era,
        connection
            .get_stakers_storage_keys_from_accounts(current_era, &left_stashes, None)
            .await
            .into_iter()
            .map(|k| k.0)
            .collect(),
    )
    .await?;
    assert_validators_are_used_as_authorities(
        &connection,
        &BTreeSet::from_iter(left_stashes.into_iter()),
        min_num_sessions,
    )
    .await;

    Ok(())
}
