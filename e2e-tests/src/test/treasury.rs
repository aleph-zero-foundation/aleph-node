use aleph_client::{
    account_from_keypair, approve_treasury_proposal, get_free_balance, make_treasury_proposal,
    reject_treasury_proposal, staking_treasury_payout, total_issuance, treasury_account,
    treasury_proposals_counter, Balance, BalanceTransfer, GetTxInfo, ReadStorage, RootConnection,
    SignedConnection, XtStatus,
};
use log::info;

use crate::{
    accounts::{get_sudo_key, get_validators_keys},
    config::Config,
    transfer::setup_for_tipped_transfer,
};

/// Returns current treasury free funds and total issuance.
///
/// Takes two storage reads.
fn balance_info<C: ReadStorage>(connection: &C) -> (Balance, Balance) {
    let treasury_balance = get_free_balance(connection, &treasury_account());
    let issuance = total_issuance(connection);
    info!(
        "[+] Treasury balance: {}. Total issuance: {}.",
        treasury_balance, issuance
    );

    (treasury_balance, issuance)
}

pub fn channeling_fee_and_tip(config: &Config) -> anyhow::Result<()> {
    let (transfer_amount, tip) = (1_000u128, 10_000u128);
    let (connection, to) = setup_for_tipped_transfer(config, tip);

    let (treasury_balance_before, issuance_before) = balance_info(&connection);
    let tx = connection.create_transfer_tx(to, transfer_amount);
    connection.transfer(tx.clone(), XtStatus::Finalized)?;
    let (treasury_balance_after, issuance_after) = balance_info(&connection);

    let possible_treasury_gain_from_staking = staking_treasury_payout(&connection);

    check_issuance(
        possible_treasury_gain_from_staking,
        issuance_before,
        issuance_after,
    );

    let fee_info = connection.get_tx_info(&tx);
    let fee = fee_info.fee_without_weight + fee_info.adjusted_weight;
    check_treasury_balance(
        possible_treasury_gain_from_staking,
        treasury_balance_before,
        treasury_balance_after,
        fee,
        tip,
    );

    Ok(())
}

fn check_issuance(
    treasury_staking_payout: Balance,
    issuance_before: Balance,
    issuance_after: Balance,
) {
    assert!(
        issuance_after >= issuance_before,
        "Unexpectedly {} was burned",
        issuance_before - issuance_after,
    );

    let diff = issuance_after - issuance_before;
    assert_eq!(
        diff % treasury_staking_payout,
        0,
        "Unexpectedly {} was minted, and it's not related to staking treasury reward which is {}",
        diff,
        treasury_staking_payout
    );
}

fn check_treasury_balance(
    possibly_treasury_gain_from_staking: Balance,
    treasury_balance_before: Balance,
    treasury_balance_after: Balance,
    fee: Balance,
    tip: Balance,
) {
    let treasury_balance_diff = treasury_balance_after - (treasury_balance_before + fee + tip);
    assert_eq!(
        treasury_balance_diff % possibly_treasury_gain_from_staking,
        0,
        "Incorrect amount was channeled to the treasury: before = {}, after = {}, fee = {}, tip = \
        {}. We can be different only as multiples of staking treasury reward {}, but the remainder \
        is {}",
        treasury_balance_before,
        treasury_balance_after,
        fee,
        tip,
        possibly_treasury_gain_from_staking,
        treasury_balance_diff % possibly_treasury_gain_from_staking,
    );
}

pub fn treasury_access(config: &Config) -> anyhow::Result<()> {
    let proposer = get_validators_keys(config)[0].clone();
    let beneficiary = account_from_keypair(&proposer);
    let connection = SignedConnection::new(&config.node, proposer);

    let proposals_counter_before = treasury_proposals_counter(&connection);
    make_treasury_proposal(&connection, 10u128, &beneficiary)?;
    make_treasury_proposal(&connection, 100u128, &beneficiary)?;
    let proposals_counter_after = treasury_proposals_counter(&connection);

    assert_eq!(
        proposals_counter_before + 2,
        proposals_counter_after,
        "Proposal has not been created"
    );

    let sudo = get_sudo_key(config);
    let connection = RootConnection::new(&config.node, sudo);

    approve_treasury_proposal(&connection, proposals_counter_after - 2)?;
    reject_treasury_proposal(&connection, proposals_counter_after - 1)?;

    Ok(())
}
