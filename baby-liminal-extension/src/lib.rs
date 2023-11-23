//! # Baby Liminal Extension
//!
//! This crate provides a way for smart contracts to interact with the [`pallet_baby_liminal`]
//! runtime module.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

// Rust features are additive, so this is the only way we can ensure that only one of these is
// enabled.
#[cfg(all(feature = "ink", feature = "runtime"))]
compile_error!("Features `ink` and `runtime` are mutually exclusive and cannot be used together");

// ------ Common stuff -----------------------------------------------------------------------------

pub mod args;
pub mod extension_ids;
pub mod status_codes;

// ------ Frontend stuff ---------------------------------------------------------------------------

#[cfg(feature = "ink")]
mod frontend;

#[cfg(feature = "ink")]
pub use frontend::{BabyLiminalError, BabyLiminalExtension, Environment};

/// Copied from `pallet_baby_liminal`.
#[cfg(feature = "ink")]
pub type VerificationKeyIdentifier = [u8; 8];

// ------ Backend stuff ----------------------------------------------------------------------------

#[cfg(feature = "runtime")]
mod backend;

#[cfg(feature = "runtime")]
pub use backend::BabyLiminalChainExtension;
#[cfg(feature = "runtime")]
pub use pallet_baby_liminal::VerificationKeyIdentifier;
