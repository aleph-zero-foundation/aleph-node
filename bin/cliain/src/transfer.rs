use aleph_client::{pallets::balances::BalanceUserApi, AccountId, SignedConnection, TxStatus};
use primitives::TOKEN;
use subxt::ext::sp_core::crypto::Ss58Codec;

pub async fn transfer(connection: SignedConnection, amount_in_tokens: u64, to_account: String) {
    let to_account = AccountId::from_ss58check(&to_account).expect("Address is valid");
    connection
        .transfer(
            to_account,
            amount_in_tokens as u128 * TOKEN,
            TxStatus::Finalized,
        )
        .await
        .unwrap();
}
