mod commands;
mod contracts;
mod keys;
mod runtime;
mod secret;
mod staking;
mod transfer;
mod validators;
mod vesting;

pub use commands::Command;
pub use contracts::{call, instantiate, instantiate_with_code, remove_code, upload_code};
pub use keys::{prepare_keys, rotate_keys, set_keys};
pub use runtime::update_runtime;
pub use secret::prompt_password_hidden;
pub use staking::{bond, force_new_era, nominate, set_staking_limits, validate};
pub use transfer::transfer;
pub use validators::change_validators;
pub use vesting::{vest, vest_other, vested_transfer};

use aleph_client::{keypair_from_string, RootConnection, SignedConnection};

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

impl From<ConnectionConfig> for SignedConnection {
    fn from(cfg: ConnectionConfig) -> Self {
        let key = keypair_from_string(&cfg.signer_seed);
        SignedConnection::new(cfg.node_endpoint.as_str(), key)
    }
}

impl From<ConnectionConfig> for RootConnection {
    fn from(cfg: ConnectionConfig) -> Self {
        RootConnection::from(Into::<SignedConnection>::into(cfg))
    }
}
