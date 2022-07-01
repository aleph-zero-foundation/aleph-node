use primitives::Balance;

use crate::AnyConnection;

/// Reads from the storage how much balance is currently on chain.
///
/// Performs a single storage read.
pub fn total_issuance<C: AnyConnection>(connection: &C) -> Balance {
    connection
        .as_connection()
        .get_storage_value("Balances", "TotalIssuance", None)
        .expect("Key `Balances::TotalIssuance` should be present in storage")
        .unwrap()
}
