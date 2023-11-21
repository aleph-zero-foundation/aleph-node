//! This is the frontend of the chain extension, i.e., the part exposed to the smart contracts.

use ink::{
    env::{DefaultEnvironment, Environment as EnvironmentT},
    prelude::vec::Vec,
    primitives::AccountId,
};

use crate::VerificationKeyIdentifier;

#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
#[allow(missing_docs)] // Error variants are self-descriptive.
/// Chain extension errors enumeration.
pub enum BabyLiminalError {
    // `pallet_baby_liminal::store_key` errors
    VerificationKeyTooLong,
    IdentifierAlreadyInUse,
    StoreKeyErrorUnknown,

    // `pallet_baby_liminal::verify` errors
    UnknownVerificationKeyIdentifier,
    DeserializingProofFailed,
    DeserializingPublicInputFailed,
    DeserializingVerificationKeyFailed,
    VerificationFailed,
    IncorrectProof,
    VerifyErrorUnknown,

    /// Couldn't serialize or deserialize data.
    ScaleError,
    /// Unexpected error code has been returned.
    UnknownError(u32),
}

impl From<scale::Error> for BabyLiminalError {
    fn from(_: scale::Error) -> Self {
        Self::ScaleError
    }
}

impl ink::env::chain_extension::FromStatusCode for BabyLiminalError {
    fn from_status_code(status_code: u32) -> Result<(), Self> {
        use crate::status_codes::*;

        match status_code {
            // Success codes
            STORE_KEY_SUCCESS | VERIFY_SUCCESS => Ok(()),

            // `pallet_baby_liminal::store_key` errors
            STORE_KEY_TOO_LONG_KEY => Err(Self::VerificationKeyTooLong),
            STORE_KEY_IDENTIFIER_IN_USE => Err(Self::IdentifierAlreadyInUse),
            STORE_KEY_ERROR_UNKNOWN => Err(Self::StoreKeyErrorUnknown),

            // `pallet_baby_liminal::verify` errors
            VERIFY_DESERIALIZING_PROOF_FAIL => Err(Self::DeserializingProofFailed),
            VERIFY_DESERIALIZING_INPUT_FAIL => Err(Self::DeserializingPublicInputFailed),
            VERIFY_UNKNOWN_IDENTIFIER => Err(Self::UnknownVerificationKeyIdentifier),
            VERIFY_DESERIALIZING_KEY_FAIL => Err(Self::DeserializingVerificationKeyFailed),
            VERIFY_VERIFICATION_FAIL => Err(Self::VerificationFailed),
            VERIFY_INCORRECT_PROOF => Err(Self::IncorrectProof),
            VERIFY_ERROR_UNKNOWN => Err(Self::VerifyErrorUnknown),

            unexpected => Err(Self::UnknownError(unexpected)),
        }
    }
}

/// BabyLiminal chain extension definition.
#[ink::chain_extension]
pub trait BabyLiminalExtension {
    type ErrorCode = BabyLiminalError;

    /// Directly call `pallet_baby_liminal::store_key`.
    // IMPORTANT: this must match the extension ID in `extension_ids.rs`! However, because constants
    // are not inlined before macro processing, we can't use an identifier from another module here.
    #[ink(extension = 41)]
    fn store_key(
        origin: AccountId,
        identifier: VerificationKeyIdentifier,
        key: Vec<u8>,
    ) -> Result<(), BabyLiminalError>;

    /// Directly call `pallet_baby_liminal::verify`.
    // IMPORTANT: this must match the extension ID in `extension_ids.rs`! However, because constants
    // are not inlined before macro processing, we can't use an identifier from another module here.
    #[ink(extension = 42)]
    fn verify(
        identifier: VerificationKeyIdentifier,
        proof: Vec<u8>,
        input: Vec<u8>,
    ) -> Result<(), BabyLiminalError>;
}

/// Default ink environment with `BabyLiminalExtension` included.
#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Environment {}

impl EnvironmentT for Environment {
    const MAX_EVENT_TOPICS: usize = <DefaultEnvironment as EnvironmentT>::MAX_EVENT_TOPICS;

    type AccountId = <DefaultEnvironment as EnvironmentT>::AccountId;
    type Balance = <DefaultEnvironment as EnvironmentT>::Balance;
    type Hash = <DefaultEnvironment as EnvironmentT>::Hash;
    type BlockNumber = <DefaultEnvironment as EnvironmentT>::BlockNumber;
    type Timestamp = <DefaultEnvironment as EnvironmentT>::Timestamp;

    type ChainExtension = BabyLiminalExtension;
}
