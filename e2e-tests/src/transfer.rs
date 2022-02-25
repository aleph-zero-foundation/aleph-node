use crate::accounts::accounts_from_seeds;
use crate::config::Config;
use crate::{Connection, TransferTransaction};
use common::create_connection;
use log::info;
use sp_core::Pair;
use sp_runtime::AccountId32;
use substrate_api_client::GenericAddress;
use substrate_api_client::{AccountId, XtStatus};

pub fn setup_for_transfer(config: &Config) -> (Connection, AccountId32, AccountId32) {
    let Config {
        ref node, seeds, ..
    } = config;

    let accounts = accounts_from_seeds(seeds);
    let (from, to) = (accounts[0].clone(), accounts[1].clone());

    let connection = create_connection(node).set_signer(from.clone());
    let from = AccountId::from(from.public());
    let to = AccountId::from(to.public());
    (connection, from, to)
}

pub fn transfer(
    target: &AccountId32,
    value: u128,
    connection: &Connection,
    exit_on: XtStatus,
) -> TransferTransaction {
    crate::send_extrinsic!(
        connection,
        "Balances",
        "transfer",
        exit_on,
        |tx_hash| info!("[+] Transfer transaction hash: {}", tx_hash),
        GenericAddress::Id(target.clone()),
        Compact(value)
    )
}
