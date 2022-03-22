use crate::{send_xt, Connection};
use pallet_staking::{RewardDestination, ValidatorPrefs};
use sp_core::Pair;
use sp_runtime::Perbill;
use substrate_api_client::{compose_call, compose_extrinsic, AccountId, GenericAddress, XtStatus};

pub fn bond(
    connection: &Connection,
    initial_stake: u128,
    controller_account_id: &AccountId,
    status: XtStatus,
) {
    let controller_account_id = GenericAddress::Id(controller_account_id.clone());

    let xt = connection.staking_bond(
        controller_account_id,
        initial_stake,
        RewardDestination::Staked,
    );
    send_xt(&connection, xt.hex_encode(), "bond", status);
}

pub fn validate(connection: &Connection, validator_commision_percentage: u8, status: XtStatus) {
    let prefs = ValidatorPrefs {
        blocked: false,
        commission: Perbill::from_percent(validator_commision_percentage as u32),
    };
    let xt = compose_extrinsic!(connection, "Staking", "validate", prefs);
    send_xt(&connection, xt.hex_encode(), "validate", status);
}

pub fn set_staking_limit(
    root_connection: &Connection,
    minimal_nominator_stake: u128,
    minimal_validator_stake: u128,
    status: XtStatus,
) {
    let set_staking_limits_call = compose_call!(
        root_connection.metadata,
        "Staking",
        "set_staking_limits",
        minimal_nominator_stake,
        minimal_validator_stake,
        0 as u8,
        0 as u8,
        0 as u8
    );
    let xt = compose_extrinsic!(root_connection, "Sudo", "sudo", set_staking_limits_call);
    send_xt(
        &root_connection,
        xt.hex_encode(),
        "set_staking_limits",
        status,
    );
}

pub fn force_new_era(root_connection: &Connection, status: XtStatus) {
    let force_new_era_call = compose_call!(root_connection.metadata, "Staking", "force_new_era");
    let xt = compose_extrinsic!(root_connection, "Sudo", "sudo", force_new_era_call);
    send_xt(&root_connection, xt.hex_encode(), "force_new_era", status);
}
