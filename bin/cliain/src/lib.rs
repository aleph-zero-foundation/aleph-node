mod keys;
mod secret;
mod staking;
mod transfer;
mod validators;

use aleph_client::{create_connection, Connection, KeyPair, Protocol};
pub use keys::{
    prepare as prepare_keys, rotate_keys_command as rotate_keys, set_keys_command as set_keys,
};
pub use secret::prompt_password_hidden;
use sp_core::Pair;
pub use staking::{
    bond_command as bond, force_new_era_command as force_new_era,
    set_staking_limits_command as set_staking_limits, validate_command as validate,
};
pub use transfer::transfer_command as transfer;
pub use validators::change as change_validators;

pub struct ConnectionConfig {
    node_endpoint: String,
    signer_seed: String,
    protocol: Protocol,
}

impl ConnectionConfig {
    pub fn new(node_endpoint: String, signer_seed: String, protocol: Protocol) -> Self {
        ConnectionConfig {
            node_endpoint,
            signer_seed,
            protocol,
        }
    }
}

impl From<ConnectionConfig> for Connection {
    fn from(cfg: ConnectionConfig) -> Self {
        let key = KeyPair::from_string(&cfg.signer_seed, None)
            .expect("Can't create pair from seed value");
        create_connection(cfg.node_endpoint.as_str(), cfg.protocol).set_signer(key)
    }
}
