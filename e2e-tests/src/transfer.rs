use aleph_client::{AccountId, Connection, KeyPair, Pair, SignedConnection};

use crate::{accounts::get_validators_raw_keys, config::Config};

async fn setup(config: &Config) -> (Connection, KeyPair, AccountId) {
    let accounts = get_validators_raw_keys(config);
    let (from, to) = (
        KeyPair::new(accounts[0].clone()),
        KeyPair::new(accounts[1].clone()),
    );
    let to = AccountId::from(to.signer().public());
    (Connection::new(&config.node).await, from, to)
}

pub async fn setup_for_transfer(config: &Config) -> (SignedConnection, AccountId) {
    let (connection, from, to) = setup(config).await;
    (SignedConnection::from_connection(connection, from), to)
}
