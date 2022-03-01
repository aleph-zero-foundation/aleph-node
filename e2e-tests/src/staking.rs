use crate::session::wait_for_session;
use crate::{send_xt, BlockNumber, Connection, KeyPair};
use common::create_connection;
use log::info;
use pallet_staking::{RewardDestination, ValidatorPrefs};
use primitives::Balance;
use sp_core::crypto::AccountId32;
use sp_core::Pair;
use sp_runtime::Perbill;
use substrate_api_client::{compose_extrinsic, AccountId, GenericAddress, XtStatus};

pub fn bond(address: &str, initial_stake: u128, stash: &KeyPair, controller: &KeyPair) {
    let connection = create_connection(address).set_signer(stash.clone());
    let controller_account_id = GenericAddress::Id(AccountId::from(controller.public()));

    let xt = connection.staking_bond(
        controller_account_id,
        initial_stake,
        RewardDestination::Staked,
    );
    send_xt(&connection, xt.hex_encode(), "bond", XtStatus::InBlock);
}

pub fn bonded(connection: &Connection, stash: &KeyPair) -> Option<AccountId> {
    let account_id = AccountId::from(stash.public());
    connection
        .get_storage_map("Staking", "Bonded", account_id, None)
        .unwrap()
}

pub fn ledger(
    connection: &Connection,
    controller: &KeyPair,
) -> Option<pallet_staking::StakingLedger<AccountId32, Balance>> {
    let account_id = AccountId::from(controller.public());
    connection
        .get_storage_map("Staking", "Ledger", account_id, None)
        .unwrap()
}

pub fn validate(address: &str, controller: &KeyPair, tx_status: XtStatus) {
    let connection = create_connection(address).set_signer(controller.clone());
    let prefs = ValidatorPrefs {
        blocked: false,
        commission: Perbill::from_percent(10),
    };

    let xt = compose_extrinsic!(connection, "Staking", "validate", prefs);
    send_xt(&connection, xt.hex_encode(), "validate", tx_status);
}

pub fn nominate(address: &str, nominator_key_pair: &KeyPair, nominee_key_pair: &KeyPair) {
    let nominee_account_id = AccountId::from(nominee_key_pair.public());
    let connection = create_connection(address).set_signer(nominator_key_pair.clone());

    let xt = connection.staking_nominate(vec![GenericAddress::Id(nominee_account_id)]);
    send_xt(&connection, xt.hex_encode(), "nominate", XtStatus::InBlock);
}

pub fn payout_stakers(address: &str, stash: KeyPair, era_number: BlockNumber) {
    let account = AccountId::from(stash.public());
    let connection = create_connection(address).set_signer(stash);
    let xt = compose_extrinsic!(connection, "Staking", "payout_stakers", account, era_number);

    send_xt(
        &connection,
        xt.hex_encode(),
        "payout_stakers",
        XtStatus::InBlock,
    );
}

pub fn get_current_era(connection: &Connection) -> u32 {
    connection
        .get_storage_value("Staking", "ActiveEra", None)
        .unwrap()
        .unwrap()
}

pub fn wait_for_full_era_completion(connection: &Connection) -> anyhow::Result<BlockNumber> {
    let current_era: u32 = get_current_era(connection);
    info!("Current era is {}", current_era);
    // staking works in such a way, that when we request a controller to be a validator in era N,
    // then the changes are applied in the era N+1 (so the new validator is receiving points in N+1),
    // so that we need N+1 to finish in order to claim the reward in era N+2 for the N+1 era
    wait_for_era_completion(connection, current_era + 2)
}

pub fn wait_for_era_completion(
    connection: &Connection,
    next_era_index: u32,
) -> anyhow::Result<BlockNumber> {
    let sessions_per_era: u32 = connection
        .get_constant("Staking", "SessionsPerEra")
        .unwrap();
    let first_session_in_next_era = next_era_index * sessions_per_era;
    wait_for_session(connection, first_session_in_next_era)?;
    Ok(next_era_index)
}
