use aleph_client::{change_members, Connection};
use log::info;
use sp_core::crypto::Ss58Codec;
use substrate_api_client::{AccountId, XtStatus};

/// Change validators to the provided list by calling the provided node.
/// The keypair has to be capable of signing sudo calls.
pub fn change(connection: Connection, validators: Vec<String>) {
    let validators = validators
        .into_iter()
        .map(|validator| AccountId::from_ss58check(&validator).expect("Address is valid"))
        .collect();

    change_members(&connection, validators, XtStatus::Finalized);

    info!("Validators changed")
}
