extern crate core;

mod commands;
mod contracts;
mod finalization;
mod keys;
mod runtime;
mod secret;
mod staking;
mod transfer;
mod treasury;
mod validators;
mod version_upgrade;
mod vesting;

use aleph_client::{keypair_from_string, Connection, RootConnection, SignedConnection};
pub use commands::Command;
pub use contracts::{
    call, instantiate, instantiate_with_code, owner_info, remove_code, upload_code,
};
pub use finalization::{finalize, set_emergency_finalizer};
pub use keys::{next_session_keys, prepare_keys, rotate_keys, set_keys};
pub use runtime::update_runtime;
pub use secret::prompt_password_hidden;
pub use staking::{bond, force_new_era, nominate, set_staking_limits, validate};
pub use transfer::transfer;
pub use treasury::{
    approve as treasury_approve, propose as treasury_propose, reject as treasury_reject,
};
pub use validators::change_validators;
pub use version_upgrade::schedule_upgrade;
pub use vesting::{vest, vest_other, vested_transfer};

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

    pub async fn get_connection(&self) -> Connection {
        Connection::new(&self.node_endpoint).await
    }

    pub async fn get_signed_connection(&self) -> SignedConnection {
        SignedConnection::new(&self.node_endpoint, keypair_from_string(&self.signer_seed)).await
    }

    pub async fn get_root_connection(&self) -> RootConnection {
        RootConnection::new(&self.node_endpoint, keypair_from_string(&self.signer_seed))
            .await
            .expect("signer should be root")
    }
}
