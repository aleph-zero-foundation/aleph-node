use ark_serialize::CanonicalDeserialize;
use jf_plonk::{
    errors::PlonkError,
    proof_system::{
        structs::{Proof, VerifyingKey},
        PlonkKzgSnark, UniversalSNARK,
    },
    transcript::StandardTranscript,
};
use pallet_contracts::Config as ContractsConfig;
use pallet_vk_storage::{Config as BabyLiminalConfig, VerificationKeys};
use scale::{Decode, Encode};
use sp_std::vec::Vec;

use crate::args::VerifyArgs;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Encode, Decode)]
pub enum ExecutorError {
    /// No verification key available under this identifier.
    UnknownVerificationKeyIdentifier,
    /// Couldn't deserialize proof.
    DeserializingProofFailed,
    /// Couldn't deserialize public input.
    DeserializingPublicInputFailed,
    /// Couldn't deserialize verification key from storage.
    DeserializingVerificationKeyFailed,
    /// Verification procedure has failed. Proof still can be correct.
    VerificationFailed,
    /// Proof has been found as incorrect.
    IncorrectProof,
}

/// Represents an 'engine' that handles chain extension calls.
pub trait BackendExecutor {
    fn verify(args: VerifyArgs) -> Result<(), ExecutorError>;
}

/// Minimal runtime configuration required by the standard chain extension executor.
pub trait MinimalRuntime: BabyLiminalConfig + ContractsConfig {}
impl<R: BabyLiminalConfig + ContractsConfig> MinimalRuntime for R {}

pub type Curve = ark_bls12_381::Bls12_381;
pub type CircuitField = ark_bls12_381::Fr;

/// Default implementation for the chain extension mechanics.
impl<Runtime: MinimalRuntime> BackendExecutor for Runtime {
    fn verify(args: VerifyArgs) -> Result<(), ExecutorError> {
        // ----------- Deserialize arguments -------------------------------------------------------
        let proof: Proof<Curve> = CanonicalDeserialize::deserialize_compressed(&*args.proof)
            .map_err(|e| {
                log::error!("Deserializing proof failed: {e:?}");
                ExecutorError::DeserializingProofFailed
            })?;

        let public_input: Vec<CircuitField> =
            CanonicalDeserialize::deserialize_compressed(&*args.public_input).map_err(|e| {
                log::error!("Deserializing public input failed: {e:?}");
                ExecutorError::DeserializingPublicInputFailed
            })?;

        // ----------- Read and deserialize verification key ---------------------------------------
        let verification_key = VerificationKeys::<Runtime>::get(args.verification_key_hash)
            .ok_or(ExecutorError::UnknownVerificationKeyIdentifier)?;

        let verification_key: VerifyingKey<Curve> =
            CanonicalDeserialize::deserialize_compressed(&**verification_key).map_err(|e| {
                log::error!("Deserializing verification key failed: {e:?}");
                ExecutorError::DeserializingVerificationKeyFailed
            })?;

        // ----------- Verify proof ----------------------------------------------------------------
        match PlonkKzgSnark::verify::<StandardTranscript>(
            &verification_key,
            &public_input,
            &proof,
            None,
        ) {
            Ok(_) => Ok(()),
            Err(PlonkError::WrongProof) => Err(ExecutorError::IncorrectProof),
            Err(_) => Err(ExecutorError::VerificationFailed),
        }
    }
}
