use std::iter::repeat;

use aleph_client::{BalanceTransfer, BatchTransactions, XtStatus};
use log::info;

use crate::{config::Config, transfer::setup_for_transfer};

pub fn batch_transactions(config: &Config) -> anyhow::Result<()> {
    const NUMBER_OF_TRANSACTIONS: usize = 100;

    let (connection, to) = setup_for_transfer(config);

    let call = connection.create_transfer_tx(to, 1000u128);
    let transactions = repeat(&call).take(NUMBER_OF_TRANSACTIONS);

    let finalized_block_hash = connection
        .batch_and_send_transactions(transactions, XtStatus::Finalized)
        .unwrap_or_else(|err| panic!("error while sending a batch of txs: {:?}", err))
        .expect("Could not get tx hash");
    info!(
        "[+] A batch of {} transactions was included in finalized {} block.",
        NUMBER_OF_TRANSACTIONS, finalized_block_hash
    );

    Ok(())
}
