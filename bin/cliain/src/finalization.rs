use std::str::FromStr;

use aleph_client::{
    emergency_finalize, finalization_set_emergency_finalizer, AlephKeyPair, BlockHash, BlockNumber,
    SignedConnection,
};
use substrate_api_client::{AccountId, XtStatus};

use crate::RootConnection;

/// Sets the emergency finalized, the provided string should be the seed phrase of the desired finalizer.
pub fn set_emergency_finalizer(connection: RootConnection, finalizer: AccountId) {
    finalization_set_emergency_finalizer(&connection, finalizer, XtStatus::Finalized)
}

/// Finalizes the given block using the key pair from provided seed as emergency finalizer.
pub fn finalize(
    connection: SignedConnection,
    number: BlockNumber,
    hash: String,
    key_pair: AlephKeyPair,
) {
    let hash = BlockHash::from_str(&hash).expect("Hash is properly hex encoded");
    emergency_finalize(&connection, number, hash, key_pair).unwrap();
}
