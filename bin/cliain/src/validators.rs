use log::info;
use aleph_client::{change_members, create_connection, KeyPair};
use substrate_api_client::{AccountId, XtStatus};
use sp_core::crypto::Ss58Codec;

/// Change validators to the provided list by calling the provided node.
/// The keypair has to be capable of signing sudo calls.
pub fn change(validators: Vec<String>, node: String, sudo_key: KeyPair) {
    let connection = create_connection(&node).set_signer(sudo_key);
    let validators: Vec<_> = validators.into_iter().map(|validator|
        AccountId::from_ss58check(&validator).expect("Address is valid")
    ).collect();

    change_members(&connection, validators, XtStatus::Finalized);

    info!("Validators changed")
}
