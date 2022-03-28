use crate::{send_xt, wait_for_session, BlockNumber, Connection};
use log::info;
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

pub fn set_staking_limits(
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

pub fn get_current_era(connection: &Connection) -> u32 {
    let current_era = connection
        .get_storage_value("Staking", "ActiveEra", None)
        .expect("Failed to decode ActiveEra extrinsic!")
        .expect("ActiveEra is empty in the storage!");
    info!(target: "aleph-client", "Current era is {}", current_era);
    current_era
}

pub fn wait_for_full_era_completion(connection: &Connection) -> anyhow::Result<BlockNumber> {
    // staking works in such a way, that when we request a controller to be a validator in era N,
    // then the changes are applied in the era N+1 (so the new validator is receiving points in N+1),
    // so that we need N+1 to finish in order to claim the reward in era N+2 for the N+1 era
    wait_for_era_completion(connection, get_current_era(connection) + 2)
}

pub fn wait_for_next_era(connection: &Connection) -> anyhow::Result<BlockNumber> {
    wait_for_era_completion(connection, get_current_era(connection) + 1)
}

fn wait_for_era_completion(
    connection: &Connection,
    next_era_index: u32,
) -> anyhow::Result<BlockNumber> {
    let sessions_per_era: u32 = connection
        .get_constant("Staking", "SessionsPerEra")
        .expect("Failed to decode SessionsPerEra extrinsic!");
    let first_session_in_next_era = next_era_index * sessions_per_era;
    wait_for_session(connection, first_session_in_next_era)?;
    Ok(next_era_index)
}
