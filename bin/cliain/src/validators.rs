use aleph_client::{change_members, Connection};
use sp_core::crypto::Ss58Codec;
use substrate_api_client::{AccountId, XtStatus};

/// Change validators to the provided list by calling the provided node.
/// The keypair has to be capable of signing sudo calls.
pub fn change_validators(root_connection: Connection, validators: Vec<String>) {
    let validators: Vec<_> = validators
        .into_iter()
        .map(|validator| AccountId::from_ss58check(&validator).expect("Address is valid"))
        .collect();

    change_members(&root_connection, validators, XtStatus::Finalized);
    // TODO we need to check state here whether change members actually succeed
    // not only here, but for all cliain commands
    // see https://cardinal-cryptography.atlassian.net/browse/AZ-699
}
