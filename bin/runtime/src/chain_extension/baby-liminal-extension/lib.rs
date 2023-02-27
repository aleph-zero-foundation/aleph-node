#![cfg_attr(not(feature = "std"), no_std)]

use ink::env::Environment;
use scale::{Decode, Encode};
#[cfg(feature = "std")]
use scale_info::TypeInfo;
use sp_std::vec::Vec;

/// Gathers all the possible errors that might occur while calling `pallet_baby_liminal::store_key` or
/// `pallet_baby_liminal::verify`.
///
/// Every variant is already documented in `pallet_baby_liminal`.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Decode, Encode)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
pub enum BabyLiminalError {
    // `pallet_baby_liminal::store_key` errors
    IdentifierAlreadyInUse,
    VerificationKeyTooLong,

    // `pallet_baby_liminal::verify` errors
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

impl ink::env::chain_extension::FromStatusCode for BabyLiminalError {
    fn from_status_code(status_code: u32) -> Result<(), Self> {
        match status_code {
            // Success codes
            10_000 | 11_000 | 12_000 => Ok(()),

            // `pallet_baby_liminal::store_key` errors
            10_001 => Err(Self::VerificationKeyTooLong),
            10_002 => Err(Self::IdentifierAlreadyInUse),

            // `pallet_baby_liminal::verify` errors
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

/// Copied from `pallet_baby_liminal`.
pub type VerificationKeyIdentifier = [u8; 4];

/// Copied from `pallet_baby_liminal`.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Decode, Encode)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
pub enum ProvingSystem {
    Groth16,
    Gm17,
    Marlin,
}

#[ink::chain_extension]
pub trait BabyLiminalExtension {
    type ErrorCode = BabyLiminalError;

    /// Directly call `pallet_baby_liminal::store_key`.
    ///
    /// The extension method ID matches the one declared in runtime:
    /// `BABY_LIMINAL_STORE_KEY_FUNC_ID`.
    #[ink(extension = 41)]
    fn store_key(identifier: VerificationKeyIdentifier, key: Vec<u8>);

    /// Directly call `pallet_baby_liminal::verify`.
    ///
    /// The extension method ID matches the one declared in runtime: `BABY_LIMINAL_VERIFY_FUNC_ID`.
    #[ink(extension = 42)]
    fn verify(
        identifier: VerificationKeyIdentifier,
        proof: Vec<u8>,
        input: Vec<u8>,
        system: ProvingSystem,
    );

    #[ink(extension = 43, handle_status = false)]
    fn poseidon_one_to_one(input: [[u64; 4]; 1]) -> [u64; 4];

    #[ink(extension = 44, handle_status = false)]
    fn poseidon_two_to_one(input: [[u64; 4]; 2]) -> [u64; 4];

    #[ink(extension = 45, handle_status = false)]
    fn poseidon_four_to_one(input: [[u64; 4]; 4]) -> [u64; 4];
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
/// All default, except `ChainExtension`, which is set to `BabyLiminalExtension`.
pub enum BabyLiminalEnvironment {}

impl Environment for BabyLiminalEnvironment {
    const MAX_EVENT_TOPICS: usize = <ink::env::DefaultEnvironment as Environment>::MAX_EVENT_TOPICS;

    type AccountId = <ink::env::DefaultEnvironment as Environment>::AccountId;
    type Balance = <ink::env::DefaultEnvironment as Environment>::Balance;
    type Hash = <ink::env::DefaultEnvironment as Environment>::Hash;
    type Timestamp = <ink::env::DefaultEnvironment as Environment>::Timestamp;
    type BlockNumber = <ink::env::DefaultEnvironment as Environment>::BlockNumber;

    type ChainExtension = BabyLiminalExtension;
}
