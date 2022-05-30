use crate::{accounts::get_validators_keys, config::Config};
use aleph_client::SignedConnection;
use sp_core::Pair;
use substrate_api_client::AccountId;

pub fn setup_for_transfer(config: &Config) -> (SignedConnection, AccountId) {
    let accounts = get_validators_keys(config);
    let (from, to) = (accounts[0].clone(), accounts[1].clone());
    let connection = SignedConnection::new(&config.node, from);
    let to = AccountId::from(to.public());
    (connection, to)
}
