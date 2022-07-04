use primitives::Balance;

use crate::AnyConnection;

/// Reads from the storage how much balance is currently on chain.
///
/// Performs a single storage read.
pub fn total_issuance<C: AnyConnection>(connection: &C) -> Balance {
    connection.read_storage_value("Balances", "TotalIssuance")
}
