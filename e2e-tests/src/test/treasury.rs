use std::thread;
use std::thread::sleep;
use std::time::Duration;

use codec::{Compact, Decode};
use common::create_connection;
use frame_support::PalletId;
use log::info;
use sp_core::Pair;
use sp_runtime::traits::AccountIdConversion;
use sp_runtime::MultiAddress;
use substrate_api_client::sp_runtime::AccountId32;
use substrate_api_client::{AccountId, UncheckedExtrinsicV4};

use crate::accounts::{accounts_from_seeds, get_free_balance, get_sudo};
use crate::config::Config;
use crate::fee::get_tx_fee_info;
use crate::transfer::{setup_for_transfer, transfer};
use crate::waiting::wait_for_event;
use crate::Connection;

pub fn channeling_fee(config: Config) -> anyhow::Result<()> {
    let (connection, _, to) = setup_for_transfer(config);
    let treasury = get_treasury_account(&connection);

    let treasury_balance_before = get_free_balance(&treasury, &connection);
    let issuance_before = get_total_issuance(&connection);
    info!(
        "[+] Treasury balance before tx: {}. Total issuance: {}.",
        treasury_balance_before, issuance_before
    );

    let tx = transfer(&to, 1000u128, &connection);

    let treasury_balance_after = get_free_balance(&treasury, &connection);
    let issuance_after = get_total_issuance(&connection);
    info!(
        "[+] Treasury balance after tx: {}. Total issuance: {}.",
        treasury_balance_after, issuance_after
    );

    assert!(
        issuance_after <= issuance_before,
        "Unexpectedly {} was minted",
        issuance_after - issuance_before
    );
    assert!(
        issuance_before <= issuance_after,
        "Unexpectedly {} was burned",
        issuance_before - issuance_after
    );

    let fee_info = get_tx_fee_info(&connection, &tx);
    let fee = fee_info.fee_without_weight + fee_info.adjusted_weight;

    assert_eq!(
        treasury_balance_before + fee,
        treasury_balance_after,
        "Incorrect amount was channeled to the treasury: before = {}, after = {}, fee = {}",
        treasury_balance_before,
        treasury_balance_after,
        fee
    );

    Ok(())
}

pub fn treasury_access(config: Config) -> anyhow::Result<()> {
    let Config { node, seeds, .. } = config.clone();

    let proposer = accounts_from_seeds(seeds)[0].to_owned();
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

fn get_treasury_account(connection: &Connection) -> AccountId32 {
    let pallet_id = connection
        .metadata
        .module_with_constants_by_name("Treasury")
        .unwrap()
        .constant_by_name("PalletId")
        .unwrap()
        .get_value();
    PalletId(pallet_id.try_into().unwrap()).into_account()
}

type ProposalTransaction =
    UncheckedExtrinsicV4<([u8; 2], Compact<u128>, MultiAddress<AccountId, ()>)>;
fn propose_treasury_spend(
    value: u128,
    beneficiary: &AccountId32,
    connection: &Connection,
) -> ProposalTransaction {
    crate::send_extrinsic!(
        connection,
        "Treasury",
        "propose_spend",
        |tx_hash| info!("[+] Treasury spend transaction hash: {}", tx_hash),
        Compact(value),
        GenericAddress::Id(beneficiary.clone())
    )
}

fn get_proposals_counter(connection: &Connection) -> u32 {
    connection
        .get_storage_value("Treasury", "ProposalCount", None)
        .unwrap()
        .unwrap()
}

type GovernanceTransaction = UncheckedExtrinsicV4<([u8; 2], Compact<u32>)>;
fn send_treasury_approval(proposal_id: u32, connection: &Connection) -> GovernanceTransaction {
    crate::send_extrinsic!(
        connection,
        "Treasury",
        "approve_proposal",
        |tx_hash| info!("[+] Treasury approval transaction hash: {}", tx_hash),
        Compact(proposal_id)
    )
}

fn treasury_approve(proposal_id: u32, connection: &Connection) -> anyhow::Result<()> {
    send_treasury_approval(proposal_id, connection);
    wait_for_approval(connection, proposal_id)
}

fn send_treasury_rejection(proposal_id: u32, connection: &Connection) -> GovernanceTransaction {
    crate::send_extrinsic!(
        connection,
        "Treasury",
        "reject_proposal",
        |tx_hash| info!("[+] Treasury rejection transaction hash: {}", tx_hash),
        Compact(proposal_id)
    )
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
