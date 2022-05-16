use aleph_client::{balances_transfer, SignedConnection};
use primitives::TOKEN;
use sp_core::crypto::Ss58Codec;
use substrate_api_client::{AccountId, XtStatus};

pub fn transfer(connection: SignedConnection, amount_in_tokens: u64, to_account: String) {
    let to_account = AccountId::from_ss58check(&to_account).expect("Address is valid");
    balances_transfer(
        &connection,
        &to_account,
        amount_in_tokens as u128 * TOKEN,
        XtStatus::Finalized,
    );
}
