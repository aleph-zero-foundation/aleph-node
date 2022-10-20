#![cfg_attr(not(feature = "std"), no_std)]

use ink_env::Environment;
use ink_lang as ink;

/// Gathers all the possible errors that might occur while calling `pallet_snarcos::store_key`.
#[derive(Copy, Clone, Eq, PartialEq, Debug, scale::Decode, scale::Encode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum StoreKeyError {
    /// This verification key identifier is already taken.
    IdentifierAlreadyInUse,
    /// Provided verification key is longer than `pallet_snarcos::MaximumVerificationKeyLength`
    /// limit.
    VerificationKeyTooLong,
    /// Unknown status code has been returned.
    ///
    /// This is to avoid panicking from status code mismatch.
    UnknownError,
}

impl ink_env::chain_extension::FromStatusCode for StoreKeyError {
    fn from_status_code(status_code: u32) -> Result<(), Self> {
        match status_code {
            0 => Ok(()),
            1 => Err(Self::VerificationKeyTooLong),
            2 => Err(Self::IdentifierAlreadyInUse),
            _ => Err(Self::UnknownError),
        }
    }
}

#[ink::chain_extension]
pub trait StoreKeyExtension {
    type ErrorCode = StoreKeyError;

    /// Directly call `pallet_snarcos::store_key`.
    ///
    /// The identifier and the key must be both mocked in the extension itself. This is
    /// a temporary simplification to avoid any problems with passing data between contract and
    /// runtime.
    ///
    /// The extension method ID matches the one declared in runtime: `SNARCOS_CHAIN_EXT`.
    #[ink(extension = 41, returns_result = false)]
    fn store_key();
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
// All default, except `ChainExtension`, which is set to `StoreKeyExtension`.
pub enum DefaultEnvironment {}

impl Environment for DefaultEnvironment {
    const MAX_EVENT_TOPICS: usize = <ink_env::DefaultEnvironment as Environment>::MAX_EVENT_TOPICS;

    type AccountId = <ink_env::DefaultEnvironment as Environment>::AccountId;
    type Balance = <ink_env::DefaultEnvironment as Environment>::Balance;
    type Hash = <ink_env::DefaultEnvironment as Environment>::Hash;
    type Timestamp = <ink_env::DefaultEnvironment as Environment>::Timestamp;
    type BlockNumber = <ink_env::DefaultEnvironment as Environment>::BlockNumber;

    type ChainExtension = StoreKeyExtension;
}
