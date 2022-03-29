use codec::Compact;
use log::info;
use pallet_staking::{RewardDestination, ValidatorPrefs};
use sp_core::Pair;
use sp_runtime::Perbill;
use substrate_api_client::{
    compose_call, compose_extrinsic, AccountId, Balance, GenericAddress, XtStatus,
};

use crate::{get_locked_balance, send_xt, wait_for_session, BlockNumber, Connection, KeyPair};

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
    send_xt(connection, xt.hex_encode(), "bond", status);
}

pub fn validate(connection: &Connection, validator_commission_percentage: u8, status: XtStatus) {
    let prefs = ValidatorPrefs {
        blocked: false,
        commission: Perbill::from_percent(validator_commission_percentage as u32),
    };
    let xt = compose_extrinsic!(connection, "Staking", "validate", prefs);
    send_xt(connection, xt.hex_encode(), "validate", status);
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
        0_u8,
        0_u8,
        0_u8
    );
    let xt = compose_extrinsic!(root_connection, "Sudo", "sudo", set_staking_limits_call);
    send_xt(
        root_connection,
        xt.hex_encode(),
        "set_staking_limits",
        status,
    );
}

pub fn force_new_era(root_connection: &Connection, status: XtStatus) {
    let force_new_era_call = compose_call!(root_connection.metadata, "Staking", "force_new_era");
    let xt = compose_extrinsic!(root_connection, "Sudo", "sudo", force_new_era_call);
    send_xt(root_connection, xt.hex_encode(), "force_new_era", status);
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

pub fn payout_stakers(
    stash_connection: &Connection,
    stash_account: &AccountId,
    era_number: BlockNumber,
) {
    let xt = compose_extrinsic!(
        stash_connection,
        "Staking",
        "payout_stakers",
        stash_account,
        era_number
    );

    send_xt(
        stash_connection,
        xt.hex_encode(),
        "payout stakers",
        XtStatus::InBlock,
    );
}

pub fn payout_stakers_and_assert_locked_balance(
    stash_connection: &Connection,
    accounts_to_check_balance: &[AccountId],
    stash_account: &AccountId,
    era: BlockNumber,
) {
    let locked_stash_balance_before_payout = accounts_to_check_balance
        .iter()
        .map(|account| get_locked_balance(account, stash_connection))
        .collect::<Vec<_>>();
    payout_stakers(stash_connection, stash_account, era - 1);
    let locked_stash_balance_after_payout = accounts_to_check_balance
        .iter()
        .map(|account| get_locked_balance(account, stash_connection))
        .collect::<Vec<_>>();
    locked_stash_balance_before_payout.iter()
        .zip(locked_stash_balance_after_payout.iter())
        .zip(accounts_to_check_balance.iter())
        .for_each(|((balance_before, balance_after), account_id)| {
            assert!(balance_after[0].amount > balance_before[0].amount,
                    "Expected payout to be positive in locked balance for account {}. Balance before: {}, balance after: {}",
                    account_id, balance_before[0].amount, balance_after[0].amount);
        }
        );
}

pub fn batch_bond(
    connection: &Connection,
    stash_controller_accounts: &[(&AccountId, &AccountId)],
    bond_value: u128,
    reward_destination: RewardDestination<GenericAddress>,
) {
    let batch_bond_calls = stash_controller_accounts
        .iter()
        .cloned()
        .map(|(stash_account, controller_account)| {
            let bond_call = compose_call!(
                connection.metadata,
                "Staking",
                "bond",
                GenericAddress::Id(controller_account.clone()),
                Compact(bond_value),
                reward_destination.clone()
            );
            compose_call!(
                connection.metadata,
                "Sudo",
                "sudo_as",
                GenericAddress::Id(stash_account.clone()),
                bond_call
            )
        })
        .collect::<Vec<_>>();

    let xt = compose_extrinsic!(connection, "Utility", "batch", batch_bond_calls);
    send_xt(
        connection,
        xt.hex_encode(),
        "batch of bond calls",
        XtStatus::InBlock,
    );
}

pub fn nominate(connection: &Connection, nominee_key_pair: &KeyPair) {
    let nominee_account_id = AccountId::from(nominee_key_pair.public());

    let xt = connection.staking_nominate(vec![GenericAddress::Id(nominee_account_id)]);
    send_xt(connection, xt.hex_encode(), "nominate", XtStatus::InBlock);
}

pub fn batch_nominate(
    connection: &Connection,
    nominator_nominee_pairs: &[(&AccountId, &AccountId)],
) {
    let batch_nominate_calls = nominator_nominee_pairs
        .iter()
        .cloned()
        .map(|(nominator, nominee)| {
            let nominate_call = compose_call!(
                connection.metadata,
                "Staking",
                "nominate",
                vec![GenericAddress::Id(nominee.clone())]
            );
            compose_call!(
                connection.metadata,
                "Sudo",
                "sudo_as",
                GenericAddress::Id(nominator.clone()),
                nominate_call
            )
        })
        .collect::<Vec<_>>();

    let xt = compose_extrinsic!(connection, "Utility", "batch", batch_nominate_calls);
    send_xt(
        connection,
        xt.hex_encode(),
        "batch of nominate calls",
        XtStatus::InBlock,
    );
}

pub fn bonded(connection: &Connection, stash: &KeyPair) -> Option<AccountId> {
    let account_id = AccountId::from(stash.public());
    connection
        .get_storage_map("Staking", "Bonded", &account_id, None)
        .unwrap_or_else(|_| panic!("Failed to obtain Bonded for account id {}", account_id))
}

pub fn ledger(
    connection: &Connection,
    controller: &KeyPair,
) -> Option<pallet_staking::StakingLedger<AccountId, Balance>> {
    let account_id = AccountId::from(controller.public());
    connection
        .get_storage_map("Staking", "Ledger", &account_id, None)
        .unwrap_or_else(|_| panic!("Failed to obtain Ledger for account id {}", account_id))
}
