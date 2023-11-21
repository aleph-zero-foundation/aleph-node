use std::ops::Deref;

use aleph_client::{
    pallets::balances::BalanceUserApi, AccountId, Connection, KeyPair, Pair, SignedConnection,
    SignedConnectionApi, TxStatus,
};
use anyhow::Result;
use primitives::Balance;
use rand::Rng;

use crate::config::Config;

/// A wrapper around a KeyPair for purposes of converting to an account id in tests.
pub struct KeyPairWrapper(KeyPair);

impl KeyPairWrapper {
    /// Creates a copy of the `connection` signed by `signer`
    pub fn sign(&self, conn: &Connection) -> SignedConnection {
        SignedConnection::from_connection(conn.clone(), self.clone().0)
    }
}

impl Clone for KeyPairWrapper {
    fn clone(&self) -> Self {
        Self(KeyPair::new(self.0.signer().clone()))
    }
}

impl Deref for KeyPairWrapper {
    type Target = KeyPair;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<&KeyPairWrapper> for AccountId {
    fn from(keypair: &KeyPairWrapper) -> Self {
        keypair.signer().public().into()
    }
}

impl From<KeyPairWrapper> for AccountId {
    fn from(keypair: KeyPairWrapper) -> Self {
        (&keypair).into()
    }
}

/// Derives a test account based on a randomized string
pub fn random_account() -> KeyPairWrapper {
    KeyPairWrapper(aleph_client::keypair_from_string(&format!(
        "//TestAccount/{}",
        rand::thread_rng().gen::<u128>()
    )))
}

/// Transfer `amount` from `from` to `to`
pub async fn transfer<S: SignedConnectionApi>(
    conn: &S,
    to: &KeyPair,
    amount: Balance,
) -> Result<()> {
    conn.transfer_keep_alive(to.signer().public().into(), amount, TxStatus::Finalized)
        .await
        .map(|_| ())
}

/// Returns a number representing the given amount of alephs (adding decimals)
pub fn alephs(basic_unit_amount: Balance) -> Balance {
    basic_unit_amount * 1_000_000_000_000
}

/// Prepares a `(conn, authority, account)` triple with some money in `account` for fees.
pub async fn basic_test_context(
    config: &Config,
) -> Result<(Connection, KeyPairWrapper, KeyPairWrapper)> {
    let conn = Connection::new(&config.node).await;
    let authority = KeyPairWrapper(aleph_client::keypair_from_string(&config.sudo_seed));
    let account = random_account();

    transfer(&authority.sign(&conn), &account, alephs(1)).await?;

    Ok((conn, authority, account))
}
