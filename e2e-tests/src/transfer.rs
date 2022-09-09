use aleph_client::{
    create_connection, AccountId, Balance, Connection, KeyPair, ManageParams, SignedConnection,
};
use sp_core::Pair;

use crate::{accounts::get_validators_keys, config::Config};

fn setup(config: &Config) -> (Connection, KeyPair, AccountId) {
    let accounts = get_validators_keys(config);
    let (from, to) = (accounts[0].clone(), accounts[1].clone());
    let to = AccountId::from(to.public());
    (create_connection(&config.node), from, to)
}

pub fn setup_for_transfer(config: &Config) -> (SignedConnection, AccountId) {
    let (connection, from, to) = setup(config);
    (SignedConnection::from_any_connection(&connection, from), to)
}

pub fn setup_for_tipped_transfer(config: &Config, tip: Balance) -> (SignedConnection, AccountId) {
    let (connection, from, to) = setup(config);
    let connection = SignedConnection::from_any_connection(&connection, from).set_tip(tip);
    (connection, to)
}
