use std::collections::BTreeSet;

use ac_primitives::ExtrinsicParams;
use aleph_client::{
    account_from_keypair, balances_batch_transfer, change_validators, get_current_session,
    keypair_from_string, rotate_keys, send_xt, set_keys, staking_bond, staking_validate,
    wait_for_full_era_completion, wait_for_session, AnyConnection, KeyPair, RootConnection,
    SignedConnection,
};
use log::info;
use pallet_elections::CommitteeSeats;
use primitives::{staking::MIN_VALIDATOR_BOND, EraIndex, TOKEN};
use sp_core::{storage::StorageKey, Pair};
use substrate_api_client::{compose_extrinsic, AccountId, XtStatus};

use crate::{accounts::get_sudo_key, Config};

/// Gathers keys and accounts for all validators used in the experiment.
struct Accounts {
    stash_keys: Vec<KeyPair>,
    stash_accounts: Vec<AccountId>,

    controller_keys: Vec<KeyPair>,
    controller_accounts: Vec<AccountId>,
}

/// Generate `Accounts` struct.
fn setup_accounts() -> Accounts {
    let seeds = (0..6).map(|idx| format!("//Validator//{}", idx));

    let stash_seeds = seeds.clone().map(|seed| format!("{}//Stash", seed));
    let stash_keys = stash_seeds.map(|s| keypair_from_string(&s));
    let stash_accounts = stash_keys.clone().map(|k| account_from_keypair(&k));

    let controller_seeds = seeds.map(|seed| format!("{}//Controller", seed));
    let controller_keys = controller_seeds.map(|s| keypair_from_string(&s));
    let controller_accounts = controller_keys.clone().map(|k| account_from_keypair(&k));

    Accounts {
        stash_keys: stash_keys.collect(),
        stash_accounts: stash_accounts.collect(),
        controller_keys: controller_keys.collect(),
        controller_accounts: controller_accounts.collect(),
    }
}

/// Endow validators (stashes and controllers), bond and rotate keys.
///
/// Signer of `connection` should have enough balance to endow new accounts.
fn prepare_validators(connection: &SignedConnection, node: &str, accounts: &Accounts) {
    balances_batch_transfer(
        connection,
        accounts.stash_accounts.clone(),
        MIN_VALIDATOR_BOND + TOKEN,
    );
    balances_batch_transfer(connection, accounts.controller_accounts.clone(), TOKEN);

    for (stash, controller) in accounts
        .stash_keys
        .iter()
        .zip(accounts.controller_accounts.iter())
    {
        let connection = SignedConnection::new(node, stash.clone());
        staking_bond(
            &connection,
            MIN_VALIDATOR_BOND,
            controller,
            XtStatus::Finalized,
        );
    }

    for controller in accounts.controller_keys.iter() {
        let keys = rotate_keys(connection).expect("Failed to generate new keys");
        let connection = SignedConnection::new(node, controller.clone());
        set_keys(&connection, keys, XtStatus::Finalized);
        staking_validate(&connection, 10, XtStatus::Finalized);
    }
}

/// Produce storage key to `ErasStakers::era`.
///
/// Since in `substrate-api-client` it seems impossible to get prefix for the first key in double
/// map, we have to do it by hand.
fn get_eras_stakers_storage_key(era: EraIndex) -> StorageKey {
    let mut bytes = sp_core::twox_128("Staking".as_bytes()).to_vec();
    bytes.extend(&sp_core::twox_128("ErasStakers".as_bytes())[..]);

    let era_encoded = codec::Encode::encode(&era);
    // `pallet_staking::ErasStakers`'s keys are `Twox64Concat`-encoded.
    let era_key: Vec<u8> = sp_core::twox_64(&era_encoded)
        .iter()
        .chain(&era_encoded)
        .cloned()
        .collect();
    bytes.extend(era_key);

    StorageKey(bytes)
}

fn stakers_as_storage_keys<C: AnyConnection>(
    connection: &C,
    accounts: &[AccountId],
    era: EraIndex,
) -> BTreeSet<StorageKey> {
    accounts
        .iter()
        .map(|acc| {
            connection
                .as_connection()
                .metadata
                .storage_double_map_key("Staking", "ErasStakers", era, acc)
                .unwrap()
        })
        .collect()
}

/// Verify that `pallet_staking::ErasStakers` contain all target validators.
///
/// We have to do it by comparing keys in storage trie.
fn assert_validators_are_elected_stakers<C: AnyConnection>(
    connection: &C,
    current_era: EraIndex,
    expected_validators_as_keys: &BTreeSet<StorageKey>,
) {
    let storage_key = get_eras_stakers_storage_key(current_era);
    let stakers = connection
        .as_connection()
        .get_keys(storage_key, None)
        .unwrap_or_else(|_| panic!("Couldn't read storage keys"))
        .unwrap_or_else(|| panic!("Couldn't read `ErasStakers` for era {}", current_era))
        .into_iter()
        .map(|key| {
            let mut bytes = [0u8; 84];
            hex::decode_to_slice(&key[2..], &mut bytes as &mut [u8]).expect("Should decode key");
            StorageKey(bytes.to_vec())
        });
    let stakers = BTreeSet::from_iter(stakers);

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
        let current_authorities: Vec<AccountId> =
            connection.read_storage_value("Session", "Validators");
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

/// Chill validator.
fn chill(connection: &SignedConnection) {
    let xt = compose_extrinsic!(connection.as_connection(), "Staking", "chill");
    send_xt(
        connection,
        xt,
        Some("chilling validator"),
        XtStatus::InBlock,
    );
}

/// Chill all validators in `chilling`.
fn chill_validators(node: &str, chilling: Vec<KeyPair>) {
    for validator in chilling.into_iter() {
        info!("Chilling validator {:?}", validator.public());
        let connection = SignedConnection::new(node, validator);
        chill(&connection);
    }
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

    let reserved_validators = accounts.stash_accounts[..3].to_vec();
    let chilling_reserved = accounts.controller_keys[0].clone();
    let non_reserved_validators = accounts.stash_accounts[3..].to_vec();
    let chilling_non_reserved = accounts.controller_keys[3].clone();

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
    let connection = SignedConnection::new(node, accounts.stash_keys[0].clone());

    let current_era = wait_for_full_era_completion(&connection)?;
    info!("New validators are in force (era: {})", current_era);

    assert_validators_are_elected_stakers(
        &connection,
        current_era,
        &stakers_as_storage_keys(&connection, &accounts.stash_accounts, current_era),
    );
    assert_validators_are_used_as_authorities(
        &connection,
        &BTreeSet::from_iter(accounts.stash_accounts.clone().into_iter()),
    );

    chill_validators(node, vec![chilling_reserved, chilling_non_reserved]);

    let current_era = wait_for_full_era_completion(&connection)?;
    info!(
        "Subset of validators should be in force (era: {})",
        current_era
    );

    let mut left_stashes = accounts.stash_accounts;
    left_stashes.remove(3);
    left_stashes.remove(0);

    assert_validators_are_elected_stakers(
        &connection,
        current_era,
        &stakers_as_storage_keys(&connection, &left_stashes, current_era),
    );
    assert_validators_are_used_as_authorities(
        &connection,
        &BTreeSet::from_iter(left_stashes.into_iter()),
    );

    Ok(())
}
