use std::{thread, thread::sleep, time::Duration};

use codec::{Compact, Decode};
use frame_support::PalletId;
use log::info;
use sp_core::Pair;
use sp_runtime::{traits::AccountIdConversion, AccountId32, MultiAddress};
use substrate_api_client::{
    compose_extrinsic, AccountId, Balance, GenericAddress, UncheckedExtrinsicV4, XtStatus,
};

use aleph_client::{
    balances_transfer, create_connection, get_free_balance, get_tx_fee_info, send_xt,
    wait_for_event, Connection,
};

use crate::{
    accounts::{accounts_from_seeds, get_sudo},
    config::Config,
    transfer::setup_for_transfer,
};

fn calculate_staking_treasury_addition(connection: &Connection) -> u128 {
    let sessions_per_era = connection
        .get_constant::<u32>("Staking", "SessionsPerEra")
        .unwrap();
    let session_period = connection
        .get_constant::<u32>("Elections", "SessionPeriod")
        .unwrap();
    let millisecs_per_block = 2 * connection
        .get_constant::<u64>("Timestamp", "MinimumPeriod")
        .unwrap();
    let millisecs_per_era = millisecs_per_block * session_period as u64 * sessions_per_era as u64;
    let treasury_era_payout_from_staking = primitives::staking::era_payout(millisecs_per_era).1;
    info!(
        "[+] Possible treasury gain from staking is {}",
        treasury_era_payout_from_staking
    );
    treasury_era_payout_from_staking
}

pub fn channeling_fee(config: &Config) -> anyhow::Result<()> {
    let (connection, _, to) = setup_for_transfer(config);
    let treasury = get_treasury_account(&connection);

    let possibly_treasury_gain_from_staking = calculate_staking_treasury_addition(&connection);
    let treasury_balance_before = get_free_balance(&connection, &treasury);
    let issuance_before = get_total_issuance(&connection);
    info!(
        "[+] Treasury balance before tx: {}. Total issuance: {}.",
        treasury_balance_before, issuance_before
    );

    let tx = balances_transfer(&connection, &to, 1000u128, XtStatus::Finalized);
    let treasury_balance_after = get_free_balance(&connection, &treasury);
    let issuance_after = get_total_issuance(&connection);
    check_treasury_issuance(
        possibly_treasury_gain_from_staking,
        treasury_balance_after,
        issuance_before,
        issuance_after,
    );

    let fee_info = get_tx_fee_info(&connection, &tx);
    let fee = fee_info.fee_without_weight + fee_info.adjusted_weight;
    check_treasury_balance(
        possibly_treasury_gain_from_staking,
        treasury_balance_before,
        treasury_balance_after,
        fee,
    );

    Ok(())
}

fn check_treasury_issuance(
    possibly_treasury_gain_from_staking: u128,
    treasury_balance_after: Balance,
    issuance_before: u128,
    issuance_after: u128,
) {
    info!(
        "[+] Treasury balance after tx: {}. Total issuance: {}.",
        treasury_balance_after, issuance_after
    );

    if issuance_after > issuance_before {
        let difference = issuance_after - issuance_before;
        assert_eq!(
            difference % possibly_treasury_gain_from_staking,
            0,
            "Unexpectedly {} was minted, and it's not related to staking treasury reward which is {}",
            difference, possibly_treasury_gain_from_staking
        );
    }

    assert!(
        issuance_before <= issuance_after,
        "Unexpectedly {} was burned",
        issuance_before - issuance_after
    );
}

fn check_treasury_balance(
    possibly_treasury_gain_from_staking: u128,
    treasury_balance_before: Balance,
    treasury_balance_after: Balance,
    fee: Balance,
) {
    let treasury_balance_diff = treasury_balance_after - (treasury_balance_before + fee);
    assert_eq!(
        treasury_balance_diff % possibly_treasury_gain_from_staking,
        0,
        "Incorrect amount was channeled to the treasury: before = {}, after = {}, fee = {}.  We can \
        be different only as multiples of staking treasury reward {}, but the remainder is {}",
        treasury_balance_before,
        treasury_balance_after,
        fee,
        possibly_treasury_gain_from_staking,
        treasury_balance_diff % possibly_treasury_gain_from_staking,
    );
}

pub fn treasury_access(config: &Config) -> anyhow::Result<()> {
    let Config {
        ref node, seeds, ..
    } = config;

    let proposer = accounts_from_seeds(seeds)[0].clone();
    let beneficiary = AccountId::from(proposer.public());
    let connection = create_connection(node).set_signer(proposer);

    propose_treasury_spend(10u128, &beneficiary, &connection);
    propose_treasury_spend(100u128, &beneficiary, &connection);
    let proposals_counter = get_proposals_counter(&connection);
    assert!(proposals_counter >= 2, "Proposal was not created");

    let sudo = get_sudo(config);
    let connection = connection.set_signer(sudo);

    treasury_approve(proposals_counter - 2, &connection)?;
    treasury_reject(proposals_counter - 1, &connection)?;

    Ok(())
}

fn get_total_issuance(connection: &Connection) -> u128 {
    connection
        .get_storage_value("Balances", "TotalIssuance", None)
        .unwrap()
        .unwrap()
}

fn get_treasury_account(_connection: &Connection) -> AccountId32 {
    PalletId(*b"a0/trsry").into_account()
}

type ProposalTransaction =
    UncheckedExtrinsicV4<([u8; 2], Compact<u128>, MultiAddress<AccountId, ()>)>;
fn propose_treasury_spend(
    value: u128,
    beneficiary: &AccountId32,
    connection: &Connection,
) -> ProposalTransaction {
    let xt = compose_extrinsic!(
        connection,
        "Treasury",
        "propose_spend",
        Compact(value),
        GenericAddress::Id(beneficiary.clone())
    );
    send_xt(
        connection,
        xt.clone(),
        Some("treasury spend"),
        XtStatus::Finalized,
    );
    xt
}

fn get_proposals_counter(connection: &Connection) -> u32 {
    connection
        .get_storage_value("Treasury", "ProposalCount", None)
        .unwrap()
        .unwrap()
}

type GovernanceTransaction = UncheckedExtrinsicV4<([u8; 2], Compact<u32>)>;

fn send_treasury_approval(proposal_id: u32, connection: &Connection) -> GovernanceTransaction {
    let xt = compose_extrinsic!(
        connection,
        "Treasury",
        "approve_proposal",
        Compact(proposal_id)
    );
    send_xt(
        connection,
        xt.clone(),
        Some("treasury approval"),
        XtStatus::Finalized,
    );
    xt
}

fn treasury_approve(proposal_id: u32, connection: &Connection) -> anyhow::Result<()> {
    send_treasury_approval(proposal_id, connection);
    wait_for_approval(connection, proposal_id)
}

fn send_treasury_rejection(proposal_id: u32, connection: &Connection) -> GovernanceTransaction {
    let xt = compose_extrinsic!(
        connection,
        "Treasury",
        "reject_proposal",
        Compact(proposal_id)
    );
    send_xt(
        connection,
        xt.clone(),
        Some("treasury rejection"),
        XtStatus::Finalized,
    );
    xt
}

fn treasury_reject(proposal_id: u32, connection: &Connection) -> anyhow::Result<()> {
    let (c, p) = (connection.clone(), proposal_id);
    let listener = thread::spawn(move || wait_for_rejection(&c, p));
    send_treasury_rejection(proposal_id, connection);
    listener.join().unwrap()
}

fn wait_for_approval(connection: &Connection, proposal_id: u32) -> anyhow::Result<()> {
    loop {
        let approvals: Vec<u32> = connection
            .get_storage_value("Treasury", "Approvals", None)
            .unwrap()
            .unwrap();
        if approvals.contains(&proposal_id) {
            info!("[+] Proposal {:?} approved successfully", proposal_id);
            return Ok(());
        } else {
            info!(
                "[+] Still waiting for approval for proposal {:?}",
                proposal_id
            );
            sleep(Duration::from_millis(500))
        }
    }
}

#[derive(Debug, Decode, Copy, Clone)]
struct ProposalRejectedEvent {
    proposal_id: u32,
    _slashed: u128,
}

fn wait_for_rejection(connection: &Connection, proposal_id: u32) -> anyhow::Result<()> {
    wait_for_event(
        connection,
        ("Treasury", "Rejected"),
        |e: ProposalRejectedEvent| {
            info!("[+] Rejected proposal {:?}", e.proposal_id);
            proposal_id.eq(&e.proposal_id)
        },
    )
    .map(|_| ())
}
