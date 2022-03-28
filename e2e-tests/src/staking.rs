use crate::transfer::locks;
use aleph_client::{send_xt, BlockNumber, Connection, KeyPair};
use codec::Compact;
use pallet_balances::BalanceLock;
pub use pallet_staking::RewardDestination;
use primitives::Balance;
use sp_core::{crypto::AccountId32, Pair};
use substrate_api_client::{compose_call, compose_extrinsic, AccountId, GenericAddress, XtStatus};

pub fn bonded(connection: &Connection, stash: &KeyPair) -> Option<AccountId> {
    let account_id = AccountId::from(stash.public());
    connection
        .get_storage_map("Staking", "Bonded", &account_id, None)
        .expect(&format!(
            "Failed to obtain Bonded for account id {}",
            account_id
        ))
}

pub fn ledger(
    connection: &Connection,
    controller: &KeyPair,
) -> Option<pallet_staking::StakingLedger<AccountId32, Balance>> {
    let account_id = AccountId::from(controller.public());
    connection
        .get_storage_map("Staking", "Ledger", &account_id, None)
        .expect(&format!(
            "Failed to obtain Ledger for account id {}",
            account_id
        ))
}

pub fn nominate(connection: &Connection, nominee_key_pair: &KeyPair) {
    let nominee_account_id = AccountId::from(nominee_key_pair.public());

    let xt = connection.staking_nominate(vec![GenericAddress::Id(nominee_account_id)]);
    send_xt(&connection, xt.hex_encode(), "nominate", XtStatus::InBlock);
}

pub fn payout_stakers(
    stash_connection: &Connection,
    stash_acount: &AccountId,
    era_number: BlockNumber,
) {
    let xt = compose_extrinsic!(
        stash_connection,
        "Staking",
        "payout_stakers",
        stash_acount,
        era_number
    );

    send_xt(
        &stash_connection,
        xt.hex_encode(),
        "payout_stakers",
        XtStatus::InBlock,
    );
}

pub fn get_locked_balance(
    stash_account: &AccountId,
    connection: &Connection,
) -> Vec<BalanceLock<Balance>> {
    let locked_stash_balance = locks(&connection, stash_account).expect(&format!(
        "Expected non-empty locked balances for account {}!",
        stash_account
    ));
    assert_eq!(
        locked_stash_balance.len(),
        1,
        "Expected locked balances for account {} to have exactly one entry!",
        stash_account
    );
    locked_stash_balance
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
    locked_stash_balance_before_payout.into_iter().zip(locked_stash_balance_after_payout.into_iter()).zip(accounts_to_check_balance.iter()).
        for_each(|((balance_before, balance_after), account_id)| {
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
        .into_iter()
        .map(|(stash_account, controller_account)| {
            let bond_call = compose_call!(
                connection.metadata,
                "Staking",
                "bond",
                GenericAddress::Id((*controller_account).clone()),
                Compact(bond_value),
                reward_destination.clone()
            );
            compose_call!(
                connection.metadata,
                "Sudo",
                "sudo_as",
                GenericAddress::Id((*stash_account).clone()),
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

pub fn batch_nominate(
    connection: &Connection,
    nominator_nominee_pairs: &[(&AccountId, &AccountId)],
) {
    let batch_nominate_calls = nominator_nominee_pairs
        .into_iter()
        .map(|(nominator, nominee)| {
            let nominate_call = compose_call!(
                connection.metadata,
                "Staking",
                "nominate",
                vec![GenericAddress::Id((*nominee).clone())]
            );
            compose_call!(
                connection.metadata,
                "Sudo",
                "sudo_as",
                GenericAddress::Id((*nominator).clone()),
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
