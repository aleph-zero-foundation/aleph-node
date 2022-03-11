use crate::transfer::locks;
use aleph_client::{
    create_connection, send_xt, wait_for_session, BlockNumber, Connection, KeyPair,
};
use codec::Compact;
use log::info;
pub use pallet_staking::RewardDestination;
use pallet_staking::ValidatorPrefs;
use primitives::Balance;
use sp_core::crypto::AccountId32;
use sp_core::Pair;
use sp_runtime::Perbill;
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
        .expect("Failed to decode ActiveEra extrinsic!")
        .expect("ActiveEra is empty in the storage!")
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
        .expect("Failed to decode SessionsPerEra extrinsic!");
    let first_session_in_next_era = next_era_index * sessions_per_era;
    wait_for_session(connection, first_session_in_next_era)?;
    Ok(next_era_index)
}

pub fn check_non_zero_payouts_for_era(
    node: &String,
    stash: &KeyPair,
    connection: &Connection,
    era: BlockNumber,
) {
    let stash_account = AccountId::from(stash.public());
    let locked_stash_balance_before_payout = locks(&connection, &stash).expect(&format!(
        "Expected non-empty locked balances for account {}!",
        stash_account
    ));
    assert_eq!(
        locked_stash_balance_before_payout.len(),
        1,
        "Expected locked balances for account {} to have exactly one entry!",
        stash_account
    );
    payout_stakers(node, stash.clone(), era - 1);
    let locked_stash_balance_after_payout = locks(&connection, &stash).expect(&format!(
        "Expected non-empty locked balances for account {}!",
        stash_account
    ));
    assert_eq!(
        locked_stash_balance_after_payout.len(),
        1,
        "Expected non-empty locked balances for account to have exactly one entry {}!",
        stash_account
    );
    assert!(locked_stash_balance_after_payout[0].amount > locked_stash_balance_before_payout[0].amount,
            "Expected payout to be positive in locked balance for account {}. Balance before: {}, balance after: {}",
            stash_account, locked_stash_balance_before_payout[0].amount, locked_stash_balance_after_payout[0].amount);
}

pub fn batch_bond(
    connection: &Connection,
    stash_controller_key_pairs: &[(&KeyPair, &KeyPair)],
    bond_value: u128,
    reward_destination: RewardDestination<GenericAddress>,
) {
    let batch_bond_calls = stash_controller_key_pairs
        .iter()
        .map(|(stash_key, controller_key)| {
            let bond_call = compose_call!(
                connection.metadata,
                "Staking",
                "bond",
                GenericAddress::Id(AccountId::from(controller_key.public())),
                Compact(bond_value),
                reward_destination.clone()
            );
            compose_call!(
                connection.metadata,
                "Sudo",
                "sudo_as",
                GenericAddress::Id(AccountId::from(stash_key.public())),
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

pub fn batch_nominate(connection: &Connection, nominator_nominee_pairs: &[(&KeyPair, &KeyPair)]) {
    let batch_nominate_calls = nominator_nominee_pairs
        .iter()
        .map(|(nominator, nominee)| {
            let nominate_call = compose_call!(
                connection.metadata,
                "Staking",
                "nominate",
                vec![GenericAddress::Id(AccountId::from(nominee.public()))]
            );
            compose_call!(
                connection.metadata,
                "Sudo",
                "sudo_as",
                GenericAddress::Id(AccountId::from(nominator.public())),
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
