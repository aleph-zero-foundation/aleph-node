use aleph_client::{
    pallets::{balances::BalanceUserApi, system::SystemApi},
    TxStatus,
};
use log::info;

use crate::{config::Config, transfer::setup_for_transfer};

pub async fn token_transfer(config: &Config) -> anyhow::Result<()> {
    let (connection, to) = setup_for_transfer(config).await;

    let balance_before = connection
        .connection
        .get_free_balance(to.clone(), None)
        .await;
    info!("[+] Account {} balance before tx: {}", to, balance_before);

    let transfer_value = 1000;
    connection
        .transfer(to.clone(), transfer_value, TxStatus::Finalized)
        .await?;

    let balance_after = connection
        .connection
        .get_free_balance(to.clone(), None)
        .await;
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
