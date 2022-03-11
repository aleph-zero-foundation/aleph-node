use aleph_client::{staking_bond, set_keys, rotate_keys, create_connection, KeyPair};
use primitives::staking::MIN_VALIDATOR_BOND;
use substrate_api_client::XtStatus;

pub fn prepare(node: String, key: KeyPair) {
    let connection = create_connection(&node).set_signer(key.clone());
    staking_bond(&connection, MIN_VALIDATOR_BOND, &key, XtStatus::Finalized);
    let new_keys = rotate_keys(&connection).expect("Connection works").expect("Received new keys");
    set_keys(&connection, new_keys, XtStatus::Finalized);
}
