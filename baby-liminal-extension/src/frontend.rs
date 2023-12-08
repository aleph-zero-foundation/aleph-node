//! This is the frontend of the chain extension, i.e., the part exposed to the smart contracts.

use ink::{
    env::{DefaultEnvironment, Environment as EnvironmentT},
    prelude::vec::Vec,
};

use crate::VerificationKeyIdentifier;

#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
#[allow(missing_docs)] // Error variants are self-descriptive.
/// Chain extension errors enumeration.
pub enum BabyLiminalError {
    // Proof verification errors.
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
            VERIFY_SUCCESS => Ok(()),

            // Proof verification errors
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

    /// Verify a ZK proof `proof` given the public input `input` against the verification key
    /// `identifier`.
    // IMPORTANT: this must match the extension ID in `extension_ids.rs`! However, because constants
    // are not inlined before macro processing, we can't use an identifier from another module here.
    #[ink(extension = 0)]
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
