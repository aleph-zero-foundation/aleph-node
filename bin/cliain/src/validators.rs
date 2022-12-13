use aleph_client::{
    pallets::elections::ElectionsSudoApi, primitives::CommitteeSeats, RootConnection, TxStatus,
};

use crate::commands::ChangeValidatorArgs;

/// Change validators to the provided list by calling the provided node.
pub async fn change_validators(
    root_connection: RootConnection,
    change_validator_args: ChangeValidatorArgs,
) {
    root_connection
        .change_validators(
            change_validator_args.reserved_validators,
            change_validator_args.non_reserved_validators,
            change_validator_args
                .committee_size
                .map(|s| CommitteeSeats {
                    reserved_seats: s.reserved_seats,
                    non_reserved_seats: s.non_reserved_seats,
                }),
            TxStatus::Finalized,
        )
        .await
        .unwrap();
    // TODO we need to check state here whether change members actually succeed
    // not only here, but for all cliain commands
    // see https://cardinal-cryptography.atlassian.net/browse/AZ-699
}
