//! This is the frontend of the chain extension, i.e., the part exposed to the smart contracts.

use ink::{
    env::{DefaultEnvironment, Environment as EnvironmentT},
    prelude::vec::Vec,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[ink::scale_derive(Encode, Decode, TypeInfo)]
#[allow(missing_docs)] // Error variants are self-descriptive.
/// Chain extension errors enumeration.
pub enum BabyLiminalError {
    // Proof verification errors.
    UnknownVerificationKeyIdentifier,
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

impl From<ink::scale::Error> for BabyLiminalError {
    fn from(_: ink::scale::Error) -> Self {
        Self::ScaleError
    }
}

impl ink::env::chain_extension::FromStatusCode for BabyLiminalError {
    fn from_status_code(status_code: u32) -> Result<(), Self> {
        use crate::status_codes::*;

        match status_code {
            // Success codes
            VERIFY_SUCCESS => Ok(()),

            // Proof verification errors
            VERIFY_DESERIALIZING_INPUT_FAIL => Err(Self::DeserializingPublicInputFailed),
            VERIFY_UNKNOWN_IDENTIFIER => Err(Self::UnknownVerificationKeyIdentifier),
            VERIFY_DESERIALIZING_KEY_FAIL => Err(Self::DeserializingVerificationKeyFailed),
            VERIFY_VERIFICATION_FAIL => Err(Self::VerificationFailed),
            VERIFY_INCORRECT_PROOF => Err(Self::IncorrectProof),

            unexpected => Err(Self::UnknownError(unexpected)),
        }
    }
}

/// BabyLiminal chain extension definition.
// IMPORTANT: this must match the extension ID in `extension_ids.rs`! However, because constants are not inlined before
// macro processing, we can't use an identifier from another module here.
#[ink::chain_extension(extension = 41)]
pub trait BabyLiminalExtension {
    type ErrorCode = BabyLiminalError;

    /// Verify a ZK proof `proof` given the public input `input` against the verification key
    /// `identifier`.
    // IMPORTANT: this must match the function ID in `extension_ids.rs`! However, because constants are not inlined
    // before macro processing, we can't use an identifier from another module here.
    #[ink(function = 0)]
    fn verify(
        identifier: crate::KeyHash,
        proof: Vec<u8>,
        input: Vec<u8>,
    ) -> Result<(), BabyLiminalError>;
}

/// Default ink environment with `BabyLiminalExtension` included.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[ink::scale_derive(Encode, Decode, TypeInfo)]
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
