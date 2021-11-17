use std::env;
use std::iter;
use std::time::Instant;

use clap::Parser;
use common::create_connection;
use log::info;
use sp_core::crypto::Ss58Codec;
use sp_core::Pair;
use substrate_api_client::{compose_call, compose_extrinsic, AccountId, XtStatus};

use config::Config;

use crate::utils::*;
use crate::waiting::{wait_for_finalized_block, wait_for_session};

mod config;
mod utils;
mod waiting;

fn main() -> anyhow::Result<()> {
    if env::var(env_logger::DEFAULT_FILTER_ENV).is_err() {
        env::set_var(env_logger::DEFAULT_FILTER_ENV, "warn");
    }
    env_logger::init();

    let config: Config = Config::parse();

    run(test_finalization, "finalization", config.clone())?;
    run(test_fee_calculation, "fee calculation", config.clone())?;
    run(test_token_transfer, "token transfer", config.clone())?;
    run(test_change_validators, "validators change", config)?;

    Ok(())
}

fn run<T>(
    testcase: fn(Config) -> anyhow::Result<T>,
    name: &str,
    config: Config,
) -> anyhow::Result<()> {
    println!("Running test: {}", name);
    let start = Instant::now();
    testcase(config).map(|_| {
        let elapsed = Instant::now().duration_since(start);
        println!("Ok! Elapsed time {}ms", elapsed.as_millis());
    })
}

fn test_finalization(config: Config) -> anyhow::Result<u32> {
    let connection = create_connection(config.node);
    wait_for_finalized_block(connection, 1)
}

fn test_fee_calculation(config: Config) -> anyhow::Result<()> {
    let Config { node, seeds, .. } = config;

    let (from, to) = get_first_two_accounts(&accounts(seeds));
    let connection = create_connection(node).set_signer(from.clone());
    let from = AccountId::from(from.public());
    let to = AccountId::from(to.public());

    let balance_before = get_free_balance(&from, &connection);
    info!("[+] Account {} balance before tx: {}", to, balance_before);

    let transfer_value = 1000u128;
    let tx = transfer(&to, transfer_value, &connection);

    let balance_after = get_free_balance(&from, &connection);
    info!("[+] Account {} balance after tx: {}", to, balance_after);

    let FeeInfo {
        fee_without_weight,
        unadjusted_weight,
        adjusted_weight,
    } = get_tx_fee_info(&connection, &tx);
    let multiplier = 1; // corresponds to `ConstantFeeMultiplierUpdate`
    assert_eq!(
        multiplier * unadjusted_weight,
        adjusted_weight,
        "Weight fee was adjusted incorrectly: raw fee = {}, adjusted fee = {}",
        unadjusted_weight,
        adjusted_weight
    );

    let expected_fee = fee_without_weight + adjusted_weight;
    assert_eq!(
        balance_before - transfer_value - expected_fee,
        balance_after,
        "Incorrect balance: before = {}, after = {}, tx = {}, expected fee = {}",
        balance_before,
        balance_after,
        transfer_value,
        expected_fee
    );

    Ok(())
}

fn test_token_transfer(config: Config) -> anyhow::Result<()> {
    let Config { node, seeds, .. } = config;

    let (from, to) = get_first_two_accounts(&accounts(seeds));
    let connection = create_connection(node).set_signer(from);
    let to = AccountId::from(to.public());

    let balance_before = get_free_balance(&to, &connection);
    info!("[+] Account {} balance before tx: {}", to, balance_before);

    let transfer_value = 1000u128;
    transfer(&to, transfer_value, &connection);

    let balance_after = get_free_balance(&to, &connection);
    info!("[+] Account {} balance after tx: {}", to, balance_after);

    assert_eq!(
        balance_before + transfer_value,
        balance_after,
        "before = {}, after = {}, tx = {}",
        balance_before,
        balance_after,
        transfer_value
    );

    Ok(())
}

fn test_change_validators(config: Config) -> anyhow::Result<()> {
    let Config { node, seeds, sudo } = config;

    let accounts = accounts(seeds);

    let sudo = match sudo {
        Some(seed) => keypair_from_string(seed),
        None => accounts.get(0).expect("whoops").to_owned(),
    };

    let connection = create_connection(node).set_signer(sudo);

    let validators_before: Vec<AccountId> = connection
        .get_storage_value("Session", "Validators", None)?
        .unwrap();

    info!("[+] Validators before tx: {:#?}", validators_before);

    let new_validators: Vec<AccountId> = accounts
        .into_iter()
        .map(|pair| pair.public().into())
        .chain(iter::once(
            AccountId::from_ss58check("5EHkv1FCd4jeQmVrbYhrETL1EAr8NJxNbukDRT4FaYWbjW8f").unwrap(),
        ))
        .collect();

    info!("[+] New validators {:#?}", new_validators);

    // wait beyond session 1
    let current_session_index = wait_for_session(connection.clone(), 1)?;
    let session_for_change = current_session_index + 2;
    info!("[+] Current session index {:?}", current_session_index);

    let call = compose_call!(
        connection.metadata,
        "Aleph",
        "change_validators",
        new_validators.clone(),
        session_for_change
    );

    let tx = compose_extrinsic!(connection, "Sudo", "sudo_unchecked_weight", call, 0_u64);

    // send and watch extrinsic until finalized
    let tx_hash = connection
        .send_extrinsic(tx.hex_encode(), XtStatus::Finalized)
        .expect("Could not send extrinsc")
        .expect("Could not get tx hash");

    info!("[+] change_validators transaction hash: {}", tx_hash);

    // wait for the change to be aplied
    wait_for_session(connection.clone(), session_for_change)?;

    let validators_after: Vec<AccountId> = connection
        .get_storage_value("Session", "Validators", None)?
        .unwrap();

    info!("[+] Validators after tx: {:#?}", validators_after);

    assert!(new_validators.eq(&validators_after));

    Ok(())
}
