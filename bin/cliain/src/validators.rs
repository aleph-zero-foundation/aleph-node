use aleph_client::RootConnection;
use substrate_api_client::XtStatus;

use crate::commands::ChangeValidatorArgs;

/// Change validators to the provided list by calling the provided node.
pub fn change_validators(
    root_connection: RootConnection,
    change_validator_args: ChangeValidatorArgs,
) {
    aleph_client::change_validators(
        &root_connection,
        change_validator_args.reserved_validators,
        change_validator_args.non_reserved_validators,
        change_validator_args.committee_size,
        XtStatus::Finalized,
    );
    // TODO we need to check state here whether change members actually succeed
    // not only here, but for all cliain commands
    // see https://cardinal-cryptography.atlassian.net/browse/AZ-699
}
