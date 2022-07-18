use aleph_client::RootConnection;
use pallet_elections::CommitteeSeats;
use sp_core::crypto::Ss58Codec;
use substrate_api_client::{AccountId, XtStatus};

/// Change validators to the provided list by calling the provided node.
pub fn change_validators(root_connection: RootConnection, validators: Vec<String>) {
    let validators: Vec<_> = validators
        .iter()
        .map(|address| AccountId::from_ss58check(address).expect("Address is valid"))
        .collect();

    let validators_len = validators.len() as u32;

    aleph_client::change_validators(
        &root_connection,
        Some(validators),
        Some(vec![]),
        Some(CommitteeSeats {
            reserved_seats: validators_len,
            non_reserved_seats: 0,
        }),
        XtStatus::Finalized,
    );
    // TODO we need to check state here whether change members actually succeed
    // not only here, but for all cliain commands
    // see https://cardinal-cryptography.atlassian.net/browse/AZ-699
}
