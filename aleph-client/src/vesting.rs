use anyhow::{ensure, Result};
use log::info;
pub use pallet_vesting::VestingInfo;
use sp_core::Pair;
use substrate_api_client::{compose_extrinsic, GenericAddress, XtStatus::Finalized};
use thiserror::Error;

use primitives::Balance;

use crate::{account_from_keypair, try_send_xt, AccountId, BlockNumber, Connection, KeyPair};

/// Gathers errors from this module.
#[derive(Debug, Error)]
pub enum VestingError {
    #[error("🦺❌ Account has no active vesting schedules.")]
    NotVesting,
    #[error("🦺❌ The connection should be signed.")]
    UnsignedConnection,
}

pub type VestingSchedule = VestingInfo<Balance, BlockNumber>;

const PALLET: &str = "Vesting";

/// Checks whether `connection` is signed. If so, returns the signer keypair.
fn ensure_signed(connection: &Connection) -> Result<KeyPair> {
    let maybe_signer = connection.signer.clone();
    ensure!(maybe_signer.is_some(), VestingError::UnsignedConnection);
    Ok(maybe_signer.expect("Must be `Some(_)`: just checked."))
}

/// Calls `pallet_vesting::vest` for the signer of `connection`, i.e. makes all unlocked balances
/// transferable.
///
/// Fails if `connection` is unsigned or transaction could not have been sent.
///
/// *Note*: This function returns `Ok(_)` even if the account has no active vesting schedules
/// and thus the extrinsic was not successful. However, semantically it is still correct.
pub fn vest(connection: Connection) -> Result<()> {
    let vester = ensure_signed(&connection)?;
    let xt = compose_extrinsic!(connection, PALLET, "vest");
    let block_hash = try_send_xt(&connection, xt, Some("Vesting"), Finalized)?
        .expect("For `Finalized` status a block hash should be returned");
    info!(
        target: "aleph-client", "Vesting for the account {:?}. Finalized in block {:?}",
        account_from_keypair(&vester), block_hash
    );
    Ok(())
}

/// Calls `pallet_vesting::vest_other` by the signer of `connection` on behalf of `vest_account`,
/// i.e. makes all unlocked balances of `vest_account` transferable.
///
/// Fails if `connection` is not signed or transaction could not have been sent.
///
/// *Note*: This function returns `Ok(_)` even if the account has no active vesting schedules
/// and thus the extrinsic was not successful. However, semantically it is still correct.
pub fn vest_other(connection: Connection, vest_account: AccountId) -> Result<()> {
    ensure_signed(&connection)?;
    let xt = compose_extrinsic!(
        connection,
        PALLET,
        "vest_other",
        GenericAddress::Id(vest_account.clone())
    );
    let block_hash = try_send_xt(&connection, xt, Some("Vesting on behalf"), Finalized)?
        .expect("For `Finalized` status a block hash should be returned");
    info!(target: "aleph-client", "Vesting on behalf of the account {:?}. Finalized in block {:?}", vest_account, block_hash);
    Ok(())
}

/// Performs a vested transfer from the signer of `connection` to `receiver` according to
/// `schedule`.
///
/// Fails if `connection` is unsigned or transaction could not have been sent.
pub fn vested_transfer(
    connection: Connection,
    receiver: AccountId,
    schedule: VestingSchedule,
) -> Result<()> {
    ensure_signed(&connection)?;
    let xt = compose_extrinsic!(
        connection,
        PALLET,
        "vested_transfer",
        GenericAddress::Id(receiver.clone()),
        schedule
    );
    let block_hash = try_send_xt(&connection, xt, Some("Vested transfer"), Finalized)?
        .expect("For `Finalized` status a block hash should be returned");
    info!(target: "aleph-client", "Vested transfer to the account {:?}. Finalized in block {:?}", receiver, block_hash);
    Ok(())
}

/// Returns all active schedules of `who`.
///
/// Fails if `who` does not have any active vesting schedules.
pub fn get_schedules(connection: &Connection, who: AccountId) -> Result<Vec<VestingSchedule>> {
    connection
        .get_storage_map::<AccountId, Option<Vec<VestingSchedule>>>(PALLET, "Vesting", who, None)?
        .flatten()
        .ok_or_else(|| VestingError::NotVesting.into())
}

/// Merges two vesting schedules (at indices `idx1` and `idx2`) of the signer of `connection`.
///
/// Fails if `connection` is unsigned or transaction could not have been sent.
///
/// *Note*: This function returns `Ok(_)` even if the account has no active vesting schedules, or
/// it has fewer schedules than `max(idx1, idx2) - 1` and thus the extrinsic was not successful.
pub fn merge_schedules(connection: Connection, idx1: u32, idx2: u32) -> Result<()> {
    let who = ensure_signed(&connection)?;
    let xt = compose_extrinsic!(connection, PALLET, "merge_schedules", idx1, idx2);
    let block_hash = try_send_xt(&connection, xt, Some("Merge vesting schedules"), Finalized)?
        .expect("For `Finalized` status a block hash should be returned");
    info!(target: "aleph-client", 
        "Merging vesting schedules (indices: {} and {}) for the account {:?}. Finalized in block {:?}", 
        idx1, idx2, account_from_keypair(&who), block_hash);
    Ok(())
}
