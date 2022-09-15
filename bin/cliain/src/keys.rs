use aleph_client::{
    get_next_session_keys, rotate_keys as rotate, rotate_keys_raw_result, set_keys as set,
    staking_bond, AnyConnection, Connection, RootConnection, SessionKeys, SignedConnection,
};
use hex::ToHex;
use log::{error, info};
use primitives::staking::MIN_VALIDATOR_BOND;
use serde_json::json;
use sp_core::crypto::Ss58Codec;
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

pub fn next_session_keys(connection: &Connection, account_id: String) {
    let account_id = AccountId::from_ss58check(&account_id).expect("Address is valid");
    match get_next_session_keys(connection, account_id) {
        Some(keys) => {
            let keys_json = json!({
                "aura": "0x".to_owned() + keys.aura.encode_hex::<String>().as_str(),
                "aleph": "0x".to_owned() + keys.aleph.encode_hex::<String>().as_str(),
            });
            println!("{}", serde_json::to_string_pretty(&keys_json).unwrap());
        }
        None => error!("No keys set for the specified account."),
    }
}
