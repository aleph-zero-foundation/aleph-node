//! # Baby Liminal Extension
//!
//! This crate provides a way for smart contracts to work with ZK proofs (SNARKs).

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]
// For testing purposes, we need to enable some unstable features.
#![cfg_attr(test, allow(incomplete_features))]
#![cfg_attr(test, feature(adt_const_params))]
#![cfg_attr(test, feature(generic_const_exprs))]

// Rust features are additive, so this is the only way we can ensure that only one of these is
// enabled.
#[cfg(all(feature = "ink", feature = "runtime"))]
compile_error!("Features `ink` and `runtime` are mutually exclusive and cannot be used together");

#[cfg(not(any(feature = "ink", feature = "runtime")))]
compile_error!("Either `ink` or `runtime` feature must be enabled (or their `-std` extensions)");

// ------ Common stuff -----------------------------------------------------------------------------

pub mod args;
pub mod extension_ids;
pub mod status_codes;

// ------ Frontend stuff ---------------------------------------------------------------------------

#[cfg(feature = "ink")]
mod frontend;

#[cfg(feature = "ink")]
pub use {
    frontend::{BabyLiminalError, BabyLiminalExtension, Environment},
    sp_core::H256 as KeyHash,
};

// ------ Backend stuff ----------------------------------------------------------------------------

#[cfg(feature = "runtime")]
mod backend;

#[cfg(feature = "runtime-benchmarks")]
pub use backend::ChainExtensionBenchmarking;
#[cfg(feature = "runtime")]
pub use {backend::BabyLiminalChainExtension, pallet_vk_storage::KeyHash};
