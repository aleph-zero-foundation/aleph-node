use aleph_client::{AnyConnection, XtStatus};
use codec::Compact;
use log::info;
use sp_core::Pair;
use substrate_api_client::{compose_call, compose_extrinsic, ExtrinsicParams, GenericAddress};

use crate::{config::Config, transfer::setup_for_transfer};

pub fn batch_transactions(config: &Config) -> anyhow::Result<()> {
    const NUMBER_OF_TRANSACTIONS: usize = 100;

    let (connection, to) = setup_for_transfer(config);

    let call = compose_call!(
        connection.as_connection().metadata,
        "Balances",
        "transfer",
        GenericAddress::Id(to),
        Compact(1000u128)
    );
    let mut transactions = Vec::new();
    for _i in 0..NUMBER_OF_TRANSACTIONS {
        transactions.push(call.clone());
    }

    let extrinsic =
        compose_extrinsic!(connection.as_connection(), "Utility", "batch", transactions);

    let finalized_block_hash = connection
        .as_connection()
        .send_extrinsic(extrinsic.hex_encode(), XtStatus::Finalized)
        .expect("Could not send extrinsc")
        .expect("Could not get tx hash");
    info!(
        "[+] A batch of {} transactions was included in finalized {} block.",
        NUMBER_OF_TRANSACTIONS, finalized_block_hash
    );

    Ok(())
}
