use std::iter::repeat;

use aleph_client::{pallets::balances::BalanceUserBatchExtApi, TxStatus};

use crate::{config::setup_test, transfer::setup_for_transfer};

#[tokio::test]
pub async fn batch_transactions() -> anyhow::Result<()> {
    let config = setup_test();
    const NUMBER_OF_TRANSACTIONS: usize = 100;

    let (connection, to) = setup_for_transfer(config).await;

    let accounts: Vec<_> = repeat(to.clone()).take(NUMBER_OF_TRANSACTIONS).collect();
    connection
        .batch_transfer(&accounts, 1000, TxStatus::Finalized)
        .await?;

    Ok(())
}
