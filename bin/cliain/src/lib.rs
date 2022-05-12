mod keys;
mod runtime;
mod secret;
mod staking;
mod transfer;
mod validators;
mod vesting;

pub use keys::{prepare_keys, rotate_keys, set_keys};
pub use runtime::update_runtime;
pub use secret::prompt_password_hidden;
pub use staking::{bond, force_new_era, set_staking_limits, validate};
pub use transfer::transfer;
pub use validators::change_validators;
pub use vesting::{vest, vest_other, vested_transfer};

use aleph_client::{create_connection, keypair_from_string, Connection};

pub struct ConnectionConfig {
    node_endpoint: String,
    signer_seed: String,
}

impl ConnectionConfig {
    pub fn new(node_endpoint: String, signer_seed: String) -> Self {
        ConnectionConfig {
            node_endpoint,
            signer_seed,
        }
    }
}

impl From<ConnectionConfig> for Connection {
    fn from(cfg: ConnectionConfig) -> Self {
        let key = keypair_from_string(&cfg.signer_seed);
        create_connection(cfg.node_endpoint.as_str()).set_signer(key)
    }
}
