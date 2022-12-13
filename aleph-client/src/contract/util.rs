//! Utilities for writing contract wrappers.

use anyhow::{anyhow, Result};
use contract_transcode::Value;
use subxt::ext::sp_core::crypto::Ss58Codec;

use crate::AccountId;

/// Returns `Ok(u128)` if the given `Value` represents one, or `Err(_)` otherwise.
///
/// ```
/// # #![feature(assert_matches)]
/// # use std::assert_matches::assert_matches;
/// # use anyhow::anyhow;
/// # use aleph_client::contract::util::to_u128;
/// use contract_transcode::Value;
///
/// assert_matches!(to_u128(Value::UInt(42)), Ok(42));
/// assert_matches!(to_u128(Value::String("not a number".to_string())), Err(_));
/// ```
pub fn to_u128(value: Value) -> Result<u128> {
    match value {
        Value::UInt(value) => Ok(value),
        _ => Err(anyhow!("Expected {:?} to be an integer", value)),
    }
}

/// Returns `Ok(AccountId)` if the given `Value` represents one, or `Err(_)` otherwise.
///
/// ```
/// # #![feature(assert_matches)]
/// # use std::assert_matches::assert_matches;
/// # use anyhow::anyhow;
/// # use aleph_client::contract::util::to_account_id;
/// use contract_transcode::Value;
///
/// assert_matches!(
///     to_account_id(Value::Literal("5H8cjBBzCJrAvDn9LHZpzzJi2UKvEGC9VeVYzWX5TrwRyVCA".to_string())),
///     Ok(_)
/// );
/// assert_matches!(to_account_id(Value::UInt(42)), Err(_));
/// ```
pub fn to_account_id(value: Value) -> Result<AccountId> {
    match value {
        Value::Literal(value) => Ok(AccountId::from_ss58check(&value)?),
        _ => Err(anyhow!("Expected {:?} to be a string", value)),
    }
}
