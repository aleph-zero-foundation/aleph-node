use aleph_client::{
    rotate_keys as rotate, rotate_keys_raw_result, set_keys as set, staking_bond, AnyConnection,
    RootConnection, SessionKeys, SignedConnection,
};
use log::info;
use primitives::staking::MIN_VALIDATOR_BOND;
use substrate_api_client::{AccountId, XtStatus};

pub fn prepare_keys(connection: RootConnection, controller_account_id: AccountId) {
    staking_bond(
        &connection.as_signed(),
        MIN_VALIDATOR_BOND,
        &controller_account_id,
        XtStatus::Finalized,
    );
    let new_keys = rotate(&connection).expect("Failed to retrieve keys");
    set(&connection.as_signed(), new_keys, XtStatus::Finalized);
}

pub fn set_keys(connection: SignedConnection, new_keys: String) {
    set(
        &connection,
        SessionKeys::try_from(new_keys).expect("Failed to parse keys"),
        XtStatus::InBlock,
    );
}

pub fn rotate_keys<C: AnyConnection>(connection: C) {
    let new_keys = rotate_keys_raw_result(&connection).expect("Failed to retrieve keys");
    info!("Rotated keys: {:?}", new_keys);
}
