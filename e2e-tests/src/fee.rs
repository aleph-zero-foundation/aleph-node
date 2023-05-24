use aleph_client::{
    api::transaction_payment::events::TransactionFeePaid,
    pallets::{balances::BalanceUserApi, fee::TransactionPaymentApi},
    sp_runtime::FixedU128,
    utility::BlocksApi,
    AccountId, SignedConnection, TxStatus,
};
use log::info;
use primitives::Balance;

pub async fn current_fees(
    connection: &SignedConnection,
    to: AccountId,
    tip: Option<Balance>,
    transfer_value: Balance,
) -> (Balance, FixedU128) {
    let actual_multiplier = connection.get_next_fee_multiplier(None).await;

    let tx_info = match tip {
        None => connection.transfer(to, transfer_value, TxStatus::Finalized),
        Some(tip) => connection.transfer_with_tip(to, transfer_value, tip, TxStatus::Finalized),
    }
    .await
    .unwrap();

    let events = connection.get_tx_events(tx_info).await.unwrap();
    let event = events.find_first::<TransactionFeePaid>().unwrap().unwrap();

    let fee = event.actual_fee;

    info!("fee payed: {}", fee);

    (fee, actual_multiplier)
}
