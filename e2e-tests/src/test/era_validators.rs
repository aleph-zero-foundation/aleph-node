use aleph_client::{
    change_validators, get_current_session, wait_for_finalized_block, wait_for_full_era_completion,
    wait_for_next_era, wait_for_session, AnyConnection, Header, KeyPair, RootConnection,
    SignedConnection,
};
use codec::Decode;
use sp_core::Pair;
use substrate_api_client::{AccountId, XtStatus};

use crate::{
    accounts::{get_sudo_key, get_validators_keys},
    Config,
};

#[derive(Decode)]
struct EraValidators {
    pub reserved: Vec<AccountId>,
    pub non_reserved: Vec<AccountId>,
}

fn get_initial_reserved_validators(config: &Config) -> Vec<KeyPair> {
    get_validators_keys(config)[..2].to_vec()
}

fn get_initial_non_reserved_validators(config: &Config) -> Vec<KeyPair> {
    get_validators_keys(config)[2..].to_vec()
}

fn get_new_reserved_validators(config: &Config) -> Vec<KeyPair> {
    get_validators_keys(config)[3..].to_vec()
}

fn get_new_non_reserved_validators(config: &Config) -> Vec<KeyPair> {
    get_validators_keys(config)[..3].to_vec()
}

fn get_pallets_reserved(
    connection: &SignedConnection,
) -> anyhow::Result<(Vec<AccountId>, Vec<AccountId>)> {
    let stored_reserved: Vec<AccountId> = connection
        .as_connection()
        .get_storage_value("Elections", "NextEraReservedValidators", None)?
        .expect("Validator storage values should be present in pallet Elections.");
    let eras_validators: EraValidators = connection
        .as_connection()
        .get_storage_value("Elections", "CurrentEraValidators", None)?
        .expect("Validator storage values should be present in pallet Elections.");

    Ok((stored_reserved, eras_validators.reserved))
}

fn get_pallets_non_reserved(
    connection: &SignedConnection,
) -> anyhow::Result<(Vec<AccountId>, Vec<AccountId>)> {
    let stored_non_reserved: Vec<AccountId> = connection
        .as_connection()
        .get_storage_value("Elections", "NextEraNonReservedValidators", None)?
        .expect("Validator storage values should be present in pallet Elections.");
    let eras_validators: EraValidators = connection
        .as_connection()
        .get_storage_value("Elections", "CurrentEraValidators", None)?
        .expect("Validator storage values should be present in pallet Elections.");

    Ok((stored_non_reserved, eras_validators.non_reserved))
}

pub fn era_validators(config: &Config) -> anyhow::Result<()> {
    let node = &config.node;
    let accounts = get_validators_keys(config);
    let sender = accounts.first().expect("Using default accounts").to_owned();
    let connection = SignedConnection::new(node, sender);

    let sudo = get_sudo_key(config);

    let root_connection = RootConnection::new(node, sudo);

    let initial_reserved_validators: Vec<_> = get_initial_reserved_validators(config)
        .iter()
        .map(|pair| AccountId::from(pair.public()))
        .collect();

    let initial_non_reserved_validators: Vec<_> = get_initial_non_reserved_validators(config)
        .iter()
        .map(|pair| AccountId::from(pair.public()))
        .collect();

    let new_non_reserved_validators: Vec<_> = get_new_non_reserved_validators(config)
        .iter()
        .map(|pair| AccountId::from(pair.public()))
        .collect();
    let new_reserved_validators: Vec<_> = get_new_reserved_validators(config)
        .iter()
        .map(|pair| AccountId::from(pair.public()))
        .collect();

    change_validators(
        &root_connection,
        Some(initial_reserved_validators.clone()),
        Some(initial_non_reserved_validators.clone()),
        Some(4),
        XtStatus::InBlock,
    );
    wait_for_full_era_completion(&connection)?;

    change_validators(
        &root_connection,
        Some(new_reserved_validators.clone()),
        Some(new_non_reserved_validators.clone()),
        Some(4),
        XtStatus::InBlock,
    );

    let current_session = get_current_session(&connection);
    wait_for_session(&connection, current_session + 1)?;

    let (stored_reserved, eras_reserved) = get_pallets_reserved(&connection)?;
    let (stored_non_reserved, eras_non_reserved) = get_pallets_non_reserved(&connection)?;

    assert_eq!(
        stored_reserved, new_reserved_validators,
        "Reserved validators' storage not properly updated after change_validators."
    );
    assert_eq!(
        eras_reserved, initial_reserved_validators,
        "Reserved validators set has been updated too early."
    );

    assert_eq!(
        stored_non_reserved, new_non_reserved_validators,
        "Non-reserved validators' storage not properly updated after change_validators."
    );
    assert_eq!(
        eras_non_reserved, initial_non_reserved_validators,
        "Non-reserved validators set has been updated too early."
    );

    wait_for_next_era(&connection)?;

    let (stored_reserved, eras_reserved) = get_pallets_reserved(&connection)?;
    let (stored_non_reserved, eras_non_reserved) = get_pallets_non_reserved(&connection)?;

    assert_eq!(
        stored_reserved, new_reserved_validators,
        "Reserved validators' storage not properly updated after change_validators."
    );
    assert_eq!(
        eras_reserved, new_reserved_validators,
        "Reserved validators set is not properly updated in the next era."
    );

    assert_eq!(
        stored_non_reserved, new_non_reserved_validators,
        "Non-reserved validators' storage not properly updated after change_validators."
    );
    assert_eq!(
        eras_non_reserved, new_non_reserved_validators,
        "Non-reserved validators set is not properly updated in the next era."
    );

    let block_number = connection
        .as_connection()
        .get_header::<Header>(None)
        .expect("Could not fetch header")
        .expect("Block exists; qed")
        .number;
    wait_for_finalized_block(&connection, block_number)?;

    Ok(())
}
