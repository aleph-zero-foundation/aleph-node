use crate::{
    TransferTransaction,
    accounts::accounts_from_seeds,
    config::Config,
};
use codec::Compact;
use aleph_client::{create_connection, send_xt, Connection, KeyPair};
use log::info;
use primitives::Balance;
use sp_core::Pair;
use sp_runtime::AccountId32;
use substrate_api_client::{compose_call, compose_extrinsic, GenericAddress, AccountId, XtStatus};

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

pub fn batch_endow_account_balances(
    connection: &Connection,
    account_keys: &[KeyPair],
    endowment: u128,
) {
    let batch_endow: Vec<_> = account_keys
        .iter()
        .map(|key| {
            compose_call!(
                connection.metadata,
                "Balances",
                "transfer",
                GenericAddress::Id(AccountId::from(key.public())),
                Compact(endowment)
            )
        })
        .collect();

    let xt = compose_extrinsic!(connection, "Utility", "batch", batch_endow);
    send_xt(
        connection,
        xt.hex_encode(),
        "batch of endow balances",
        XtStatus::InBlock,
    );
}

pub fn locks(
    connection: &Connection,
    account: &KeyPair,
) -> Option<Vec<pallet_balances::BalanceLock<Balance>>> {
    let account_id = AccountId::from(account.public());
    connection
        .get_storage_map("Balances", "Locks", account_id, None)
        .unwrap()
}
