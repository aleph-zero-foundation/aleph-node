/// Pallet aleph API
pub mod aleph;
/// Pallet author API
pub mod author;
/// Pallet baby liminal API
#[cfg(feature = "liminal")]
pub mod baby_liminal;
/// Pallet balances API
pub mod balances;
/// Pallet contracts API
pub mod contract;
/// Pallet elections API
pub mod elections;
/// Pallet transaction payment API
pub mod fee;
/// Pallet multisig API
pub mod multisig;
/// Pallet session API
pub mod session;
/// Pallet staking API
pub mod staking;
/// Pallet system API
pub mod system;
/// Pallet treasury API
pub mod treasury;
/// Pallet utility API
pub mod utility;
/// Pallet vesting API
pub mod vesting;
