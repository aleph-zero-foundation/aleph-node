use crate::{accounts::accounts_from_seeds, config::Config};
use aleph_client::{create_connection, Connection, KeyPair};
use sp_core::Pair;
use substrate_api_client::AccountId;

pub fn setup_for_transfer(config: &Config) -> (Connection, KeyPair, AccountId) {
    let Config {
        ref node, seeds, ..
    } = config;

    let accounts = accounts_from_seeds(seeds);
    let (from, to) = (accounts[0].clone(), accounts[1].clone());
    let connection = create_connection(node).set_signer(from.clone());
    let to = AccountId::from(to.public());
    (connection, from, to)
}
