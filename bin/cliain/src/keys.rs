use aleph_client::{rotate_keys, set_keys, staking_bond, Connection, KeyPair};
use primitives::staking::MIN_VALIDATOR_BOND;
use substrate_api_client::XtStatus;

pub fn prepare(connection: Connection, key: KeyPair) {
    staking_bond(&connection, MIN_VALIDATOR_BOND, &key, XtStatus::Finalized);
    let new_keys = rotate_keys(&connection)
        .expect("Connection works")
        .expect("Received new keys");
    set_keys(&connection, new_keys, XtStatus::Finalized);
}
