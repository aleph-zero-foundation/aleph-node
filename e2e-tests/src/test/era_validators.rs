use aleph_client::{
    change_validators, get_current_block_number, get_current_era_non_reserved_validators,
    get_current_era_reserved_validators, get_current_session, get_next_era_non_reserved_validators,
    get_next_era_reserved_validators, wait_for_finalized_block, wait_for_full_era_completion,
    wait_for_next_era, wait_for_session, AccountId, KeyPair, SignedConnection, XtStatus,
};
use primitives::CommitteeSeats;

use crate::{
    accounts::{account_ids_from_keys, get_validators_keys},
    Config,
};

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

fn get_current_and_next_era_reserved_validators(
    connection: &SignedConnection,
) -> (Vec<AccountId>, Vec<AccountId>) {
    let stored_reserved = get_next_era_reserved_validators(connection);
    let current_reserved = get_current_era_reserved_validators(connection);
    (current_reserved, stored_reserved)
}

fn get_current_and_next_era_non_reserved_validators(
    connection: &SignedConnection,
) -> (Vec<AccountId>, Vec<AccountId>) {
    let stored_non_reserved = get_next_era_non_reserved_validators(connection);
    let current_non_reserved = get_current_era_non_reserved_validators(connection);
    (current_non_reserved, stored_non_reserved)
}

pub fn era_validators(config: &Config) -> anyhow::Result<()> {
    let connection = config.get_first_signed_connection();
    let root_connection = config.create_root_connection();

    let initial_reserved_validators_keys = get_initial_reserved_validators(config);
    let initial_reserved_validators = account_ids_from_keys(&initial_reserved_validators_keys);

    let initial_non_reserved_validators_keys = get_initial_non_reserved_validators(config);
    let initial_non_reserved_validators =
        account_ids_from_keys(&initial_non_reserved_validators_keys);

    let new_reserved_validators_keys = get_new_reserved_validators(config);
    let new_reserved_validators = account_ids_from_keys(&new_reserved_validators_keys);

    let new_non_reserved_validators_keys = get_new_non_reserved_validators(config);
    let new_non_reserved_validators = account_ids_from_keys(&new_non_reserved_validators_keys);

    change_validators(
        &root_connection,
        Some(initial_reserved_validators.clone()),
        Some(initial_non_reserved_validators.clone()),
        Some(CommitteeSeats {
            reserved_seats: 2,
            non_reserved_seats: 2,
        }),
        XtStatus::InBlock,
    );
    wait_for_full_era_completion(&connection)?;

    change_validators(
        &root_connection,
        Some(new_reserved_validators.clone()),
        Some(new_non_reserved_validators.clone()),
        Some(CommitteeSeats {
            reserved_seats: 2,
            non_reserved_seats: 2,
        }),
        XtStatus::InBlock,
    );

    let current_session = get_current_session(&connection);
    wait_for_session(&connection, current_session + 1)?;

    let (eras_reserved, stored_reserved) =
        get_current_and_next_era_reserved_validators(&connection);
    let (eras_non_reserved, stored_non_reserved) =
        get_current_and_next_era_non_reserved_validators(&connection);

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

    let (eras_reserved, stored_reserved) =
        get_current_and_next_era_reserved_validators(&connection);
    let (eras_non_reserved, stored_non_reserved) =
        get_current_and_next_era_non_reserved_validators(&connection);

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

    let block_number = get_current_block_number(&connection);
    wait_for_finalized_block(&connection, block_number)?;

    Ok(())
}
