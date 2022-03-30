use aleph_client::{
    rotate_keys as rotate, rotate_keys_raw_result, set_keys as set, staking_bond, Connection,
    SessionKeys,
};
use log::info;
use primitives::staking::MIN_VALIDATOR_BOND;
use substrate_api_client::{AccountId, XtStatus};

pub fn prepare_keys(connection: Connection, controller_account_id: AccountId) {
    staking_bond(
        &connection,
        MIN_VALIDATOR_BOND,
        &controller_account_id,
        XtStatus::Finalized,
    );
    let new_keys = rotate(&connection).expect("Failed to retrieve keys");
    set(&connection, new_keys, XtStatus::Finalized);
}

pub fn set_keys(connection: Connection, new_keys: String) {
    set(
        &connection,
        SessionKeys::try_from(new_keys).expect("Failed to parse keys"),
        XtStatus::InBlock,
    );
}

pub fn rotate_keys(connection: Connection) {
    let new_keys = rotate_keys_raw_result(&connection).expect("Failed to retrieve keys");
    info!("Rotated keys: {:?}", new_keys);
}
