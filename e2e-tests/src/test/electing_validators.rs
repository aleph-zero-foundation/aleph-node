use std::collections::BTreeSet;

use aleph_client::{
    change_validators, get_current_session, get_current_validators, get_eras_stakers_storage_key,
    get_stakers_as_storage_keys, get_stakers_as_storage_keys_from_storage_key,
    staking_chill_all_validators, wait_for_full_era_completion, wait_for_session, AccountId,
    AnyConnection, RootConnection, SignedConnection, XtStatus,
};
use log::info;
use primitives::{CommitteeSeats, EraIndex};
use sp_core::storage::StorageKey;

use crate::{
    accounts::get_sudo_key,
    validators::{prepare_validators, setup_accounts},
    Config,
};

/// Verify that `pallet_staking::ErasStakers` contains all target validators.
///
/// We have to do it by comparing keys in storage trie.
fn assert_validators_are_elected_stakers<C: AnyConnection>(
    connection: &C,
    current_era: EraIndex,
    expected_validators_as_keys: &BTreeSet<StorageKey>,
) {
    let storage_key = get_eras_stakers_storage_key(current_era);
    let stakers =
        get_stakers_as_storage_keys_from_storage_key(connection, current_era, storage_key);

    assert_eq!(
        *expected_validators_as_keys, stakers,
        "Expected another set of staking validators.\n\tExpected: {:?}\n\tActual: {:?}",
        expected_validators_as_keys, stakers
    );
}

/// Verify that all target validators are included `pallet_session::Validators` across a few
/// consecutive sessions.
fn assert_validators_are_used_as_authorities<C: AnyConnection>(
    connection: &C,
    expected_authorities: &BTreeSet<AccountId>,
) {
    let mut authorities = BTreeSet::new();
    // There are 4 slots, 3 reserved validators and 3 nonreserved, therefore after 3 sessions
    // we should observe all of them.
    for _ in 0..3 {
        let current_session = get_current_session(connection);

        info!("Reading authorities in session {}", current_session);
        let current_authorities = get_current_validators(connection);
        for ca in current_authorities.into_iter() {
            authorities.insert(ca);
        }

        wait_for_session(connection, current_session + 1).expect("Couldn't wait for next session");
    }

    assert_eq!(
        *expected_authorities, authorities,
        "Expected another set of authorities.\n\tExpected: {:?}\n\tActual: {:?}",
        expected_authorities, authorities
    );
}

/// 1. Setup 6 brand new validators - 3 reserved and 3 non reserved.
/// 2. Wait until they are in force.
/// 3. 1 reserved and 1 non reserved chill.
/// 4. Verify only staking validators are in force.
///
/// Note:
///  - `pallet_staking` has `MinValidatorCount` set to 4 (and this cannot be changed on running
///    chain)
///  - our e2e tests run with 5 validators
/// Thus chilling 2 validators (1 reserved and 1 non reserved) is a no go: `pallet_staking` will
/// protest and won't proceed with a new committee. Therefore we have to create a new, bigger
/// committee. This is much easier to maintain with a fresh set of accounts. However, after
/// generating new keys for new members (with `rotate_keys`), **FINALIZATION IS STALLED**. This is
/// because a single node keeps in its keystore all Aleph keys, which is neither expected nor
/// handled by our code. Fortunately, Aura handles this gently, so after changing committee block
/// production keeps working. This is completetly enough for this test.
pub fn authorities_are_staking(config: &Config) -> anyhow::Result<()> {
    let node = &config.node;
    let sudo = get_sudo_key(config);
    let root_connection = RootConnection::new(node, sudo);

    let accounts = setup_accounts();
    prepare_validators(&root_connection.as_signed(), node, &accounts);
    info!("New validators are set up");

    let reserved_validators = accounts.get_stash_accounts()[..3].to_vec();
    let chilling_reserved = accounts.get_controller_keys()[0].clone();
    let non_reserved_validators = accounts.get_stash_accounts()[3..].to_vec();
    let chilling_non_reserved = accounts.get_controller_keys()[3].clone();

    change_validators(
        &root_connection,
        Some(reserved_validators),
        Some(non_reserved_validators),
        Some(CommitteeSeats {
            reserved_seats: 3,
            non_reserved_seats: 1,
        }),
        XtStatus::Finalized,
    );
    info!("Changed validators to a new set");

    // We need any signed connection.
    let connection = SignedConnection::new(node, accounts.get_stash_keys()[0].clone());

    let current_era = wait_for_full_era_completion(&connection)?;
    info!("New validators are in force (era: {})", current_era);

    assert_validators_are_elected_stakers(
        &connection,
        current_era,
        &get_stakers_as_storage_keys(&connection, accounts.get_stash_accounts(), current_era),
    );
    assert_validators_are_used_as_authorities(
        &connection,
        &BTreeSet::from_iter(accounts.get_stash_accounts().clone().into_iter()),
    );

    staking_chill_all_validators(node, vec![chilling_reserved, chilling_non_reserved]);

    let current_era = wait_for_full_era_completion(&connection)?;
    info!(
        "Subset of validators should be in force (era: {})",
        current_era
    );

    let mut left_stashes = accounts.get_stash_accounts().clone();
    left_stashes.remove(3);
    left_stashes.remove(0);

    assert_validators_are_elected_stakers(
        &connection,
        current_era,
        &get_stakers_as_storage_keys(&connection, &left_stashes, current_era),
    );
    assert_validators_are_used_as_authorities(
        &connection,
        &BTreeSet::from_iter(left_stashes.into_iter()),
    );

    Ok(())
}
