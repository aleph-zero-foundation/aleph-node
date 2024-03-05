use core::marker::ConstParamTy;

use crate::{args::VerifyArgs, backend::executor::BackendExecutor};

#[derive(ConstParamTy, Copy, Clone, Eq, PartialEq, Debug)]
pub enum VerifierError {
    UnknownVerificationKeyIdentifier,
    DeserializingPublicInputFailed,
    DeserializingVerificationKeyFailed,
    VerificationFailed,
    IncorrectProof,
}

/// Describes how the `Executor` should behave when one of its methods is called.
#[derive(ConstParamTy, Clone, Eq, PartialEq)]
pub enum Responder {
    /// Twist and shout.
    Panicker,
    /// Return `Ok(())`.
    Okayer,
    /// Return `Err(Error)`.
    Errorer(VerifierError),
}

/// Auxiliary method to construct type argument.
///
/// Due to "`struct/enum construction is not supported in generic constants`".
pub const fn make_errorer<const ERROR: VerifierError>() -> Responder {
    Responder::Errorer(ERROR)
}

/// A testing counterpart for `Runtime`.
///
/// `VERIFY_RESPONDER` instructs how to behave when `verify` is called.
pub struct MockedExecutor<const VERIFY_RESPONDER: Responder>;

/// Executor that will scream when `verify` is called.
pub type Panicker = MockedExecutor<{ Responder::Panicker }>;

/// Executor that will return `Ok(())` for `verify`.
pub type VerifyOkayer = MockedExecutor<{ Responder::Okayer }>;

/// Executor that will return `Err(ERROR)` for `verify`.
pub type VerifyErrorer<const ERROR: VerifierError> = MockedExecutor<{ make_errorer::<ERROR>() }>;

impl<const VERIFY_RESPONDER: Responder> BackendExecutor for MockedExecutor<VERIFY_RESPONDER> {
    fn verify(
        _: VerifyArgs,
    ) -> Result<(), aleph_runtime_interfaces::snark_verifier::VerifierError> {
        match VERIFY_RESPONDER {
            Responder::Panicker => panic!("Function `verify` shouldn't have been executed"),
            Responder::Okayer => Ok(()),
            Responder::Errorer(e) => {
                use aleph_runtime_interfaces::snark_verifier::VerifierError::*;
                match e {
                    VerifierError::UnknownVerificationKeyIdentifier => {
                        Err(UnknownVerificationKeyIdentifier)
                    }
                    VerifierError::DeserializingPublicInputFailed => {
                        Err(DeserializingPublicInputFailed)
                    }
                    VerifierError::DeserializingVerificationKeyFailed => {
                        Err(DeserializingVerificationKeyFailed)
                    }
                    VerifierError::VerificationFailed => Err(VerificationFailed),
                    VerifierError::IncorrectProof => Err(IncorrectProof),
                }
            }
        }
    }
}
