use aleph_client::{
    pallets::balances::BalanceUserApi, AccountId, Balance, SignedConnection, Ss58Codec, TxStatus,
};
use primitives::TOKEN;

pub async fn transfer_keep_alive(
    connection: SignedConnection,
    amount_in_tokens: u64,
    to_account: String,
) {
    let to_account = AccountId::from_ss58check(&to_account).expect("Address is valid");
    connection
        .transfer_keep_alive(
            to_account,
            amount_in_tokens as Balance * TOKEN,
            TxStatus::Finalized,
        )
        .await
        .unwrap();
}
