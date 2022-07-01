use aleph_client::{
    approve_treasury_proposal, make_treasury_proposal, reject_treasury_proposal, RootConnection,
    SignedConnection,
};
use primitives::{Balance, TOKEN};
use sp_core::crypto::Ss58Codec;
use substrate_api_client::AccountId;

/// Delegates to `aleph_client::make_treasury_proposal`.
pub fn propose(connection: SignedConnection, amount_in_tokens: u64, beneficiary: String) {
    let beneficiary = AccountId::from_ss58check(&beneficiary).expect("Address should be valid");
    let endowment = amount_in_tokens as Balance * TOKEN;

    make_treasury_proposal(&connection, endowment, &beneficiary)
        .expect("Should successfully make a proposal");
}

/// Delegates to `aleph_client::approve_treasury_proposal`.
pub fn approve(connection: RootConnection, proposal_id: u32) {
    approve_treasury_proposal(&connection, proposal_id)
        .expect("Should successfully approve the proposal")
}

/// Delegates to `aleph_client::reject_treasury_proposal`.
pub fn reject(connection: RootConnection, proposal_id: u32) {
    reject_treasury_proposal(&connection, proposal_id)
        .expect("Should successfully reject the proposal")
}
