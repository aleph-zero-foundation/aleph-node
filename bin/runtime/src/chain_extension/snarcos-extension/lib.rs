#![cfg_attr(not(feature = "std"), no_std)]

use ink_env::Environment;
use ink_lang as ink;
use scale::{Decode, Encode};
#[cfg(feature = "std")]
use scale_info::TypeInfo;
use sp_std::vec::Vec;

/// Gathers all the possible errors that might occur while calling `pallet_snarcos::store_key` or
/// `pallet_snarcos::verify`.
///
/// Every variant is already documented in `pallet_snarcos`.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Decode, Encode)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
pub enum SnarcosError {
    // `pallet_snarcos::store_key` errors
    IdentifierAlreadyInUse,
    VerificationKeyTooLong,

    // `pallet_snarcos::verify` errors
    UnknownVerificationKeyIdentifier,
    DeserializingProofFailed,
    DeserializingPublicInputFailed,
    DeserializingVerificationKeyFailed,
    VerificationFailed,
    IncorrectProof,

    /// Unknown status code has been returned.
    ///
    /// This is to avoid panicking from status code mismatch.
    UnknownError,
}

impl ink_env::chain_extension::FromStatusCode for SnarcosError {
    fn from_status_code(status_code: u32) -> Result<(), Self> {
        match status_code {
            // Success codes
            10_000 | 11_000 => Ok(()),

            // `pallet_snarcos::store_key` errors
            10_001 => Err(Self::VerificationKeyTooLong),
            10_002 => Err(Self::IdentifierAlreadyInUse),

            // `pallet_snarcos::verify` errors
            11_001 => Err(Self::DeserializingProofFailed),
            11_002 => Err(Self::DeserializingPublicInputFailed),
            11_003 => Err(Self::UnknownVerificationKeyIdentifier),
            11_004 => Err(Self::DeserializingVerificationKeyFailed),
            11_005 => Err(Self::VerificationFailed),
            11_006 => Err(Self::IncorrectProof),

            _ => Err(Self::UnknownError),
        }
    }
}

/// Copied from `pallet_snarcos`.
pub type VerificationKeyIdentifier = [u8; 4];

/// Copied from `pallet_snarcos`.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Decode, Encode)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
pub enum ProvingSystem {
    Groth16,
    Gm17,
}

#[ink::chain_extension]
pub trait SnarcosExtension {
    type ErrorCode = SnarcosError;

    /// Directly call `pallet_snarcos::store_key`.
    ///
    /// The extension method ID matches the one declared in runtime: `SNARCOS_STORE_KEY_FUNC_ID`.
    #[ink(extension = 41, returns_result = false)]
    fn store_key(identifier: VerificationKeyIdentifier, key: Vec<u8>);

    /// Directly call `pallet_snarcos::verify`.
    ///
    /// The extension method ID matches the one declared in runtime: `SNARCOS_VERIFY_FUNC_ID`.
    #[ink(extension = 42, returns_result = false)]
    fn verify(
        identifier: VerificationKeyIdentifier,
        proof: Vec<u8>,
        input: Vec<u8>,
        system: ProvingSystem,
    );
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
/// All default, except `ChainExtension`, which is set to `SnarcosExtension`.
pub enum DefaultEnvironment {}

impl Environment for DefaultEnvironment {
    const MAX_EVENT_TOPICS: usize = <ink_env::DefaultEnvironment as Environment>::MAX_EVENT_TOPICS;

    type AccountId = <ink_env::DefaultEnvironment as Environment>::AccountId;
    type Balance = <ink_env::DefaultEnvironment as Environment>::Balance;
    type Hash = <ink_env::DefaultEnvironment as Environment>::Hash;
    type Timestamp = <ink_env::DefaultEnvironment as Environment>::Timestamp;
    type BlockNumber = <ink_env::DefaultEnvironment as Environment>::BlockNumber;

    type ChainExtension = SnarcosExtension;
}
