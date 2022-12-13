use std::str::FromStr;

use aleph_client::{
    pallets::aleph::{AlephRpc, AlephSudoApi},
    AccountId, AlephKeyPair, Connection, TxStatus,
};
use primitives::{BlockHash, BlockNumber};

use crate::RootConnection;

/// Sets the emergency finalized, the provided string should be the seed phrase of the desired finalizer.
pub async fn set_emergency_finalizer(connection: RootConnection, finalizer: AccountId) {
    connection
        .set_emergency_finalizer(finalizer, TxStatus::Finalized)
        .await
        .unwrap();
}

/// Finalizes the given block using the key pair from provided seed as emergency finalizer.
pub async fn finalize(
    connection: Connection,
    number: BlockNumber,
    hash: String,
    key_pair: AlephKeyPair,
) {
    let hash = BlockHash::from_str(&hash).expect("Hash is properly hex encoded");
    connection
        .emergency_finalize(number, hash, key_pair)
        .await
        .unwrap();
}
