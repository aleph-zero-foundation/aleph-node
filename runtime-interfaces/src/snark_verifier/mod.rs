//! An interface that provides to the runtime a functionality of verifying halo2 SNARKs, together with related errors
//! and configuration.

#[cfg(feature = "std")]
mod implementation;
#[cfg(all(test, feature = "std"))]
mod tests;

#[cfg(feature = "std")]
pub use implementation::{Curve, Fr, G1Affine};
// Reexport `verify` and `HostFunctions`, so that they are not imported like
// `aleph-runtime-interfaces::snark_verifier::snark_verifier::<>`.
pub use snark_verifier::verify;
#[cfg(feature = "std")]
pub use snark_verifier::HostFunctions;

/// Log_2(max number of rows in a supported circuit).
///
/// Note: the same constant MUST be used in the params generation for preparing proving and
/// verifying keys.
pub const CIRCUIT_MAX_K: u32 = 12;

/// Gathers errors that can happen during proof verification.
#[derive(Copy, Clone, Eq, PartialEq, Debug, codec::Encode, codec::Decode)]
pub enum VerifierError {
    /// No verification key available under this identifier.
    UnknownVerificationKeyIdentifier,
    /// Couldn't deserialize public input.
    DeserializingPublicInputFailed,
    /// Couldn't deserialize verification key from storage.
    DeserializingVerificationKeyFailed,
    /// Verification procedure has failed. Proof still can be correct.
    VerificationFailed,
    /// Proof has been found as incorrect.
    IncorrectProof,
}

/// An interface that provides to the runtime a functionality of verifying halo2 SNARKs.
#[sp_runtime_interface::runtime_interface]
pub trait SnarkVerifier {
    /// Verify `proof` given `verifying_key`.
    fn verify(
        proof: &[u8],
        public_input: &[u8],
        verifying_key: &[u8],
    ) -> Result<(), VerifierError> {
        #[cfg(not(feature = "std"))]
        unreachable!("Runtime interface implementation is not available in the no-std mode");

        #[cfg(feature = "std")]
        implementation::do_verify(proof, public_input, verifying_key)
    }
}
