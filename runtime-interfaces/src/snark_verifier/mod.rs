//! An interface that provides to the runtime a functionality of verifying halo2 SNARKs, together with related errors
//! and configuration.

#[cfg(feature = "std")]
mod implementation;
#[cfg(all(test, feature = "std"))]
mod tests;

#[cfg(feature = "std")]
pub use implementation::{Curve, Fr, G1Affine};
use parity_scale_codec::{Decode, Encode};
// Reexport `verify` and `HostFunctions`, so that they are not imported like
// `aleph-runtime-interfaces::snark_verifier::snark_verifier::<>`.
pub use snark_verifier::verify;
#[cfg(feature = "std")]
pub use snark_verifier::HostFunctions;

/// Gathers errors that can happen during proof verification.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Encode, Decode)]
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

/// Serializes `vk` together with `k` into a vector of bytes.
///
/// A corresponding deserialization procedure is implemented in the verifier.
#[cfg(feature = "std")]
pub fn serialize_vk(vk: halo2_proofs::plonk::VerifyingKey<G1Affine>, k: u32) -> Vec<u8> {
    let mut buffer = Vec::new();
    buffer.extend(k.to_le_bytes());
    // We use `SerdeFormat::RawBytesUnchecked` here for performance reasons.
    buffer.extend(vk.to_bytes(halo2_proofs::SerdeFormat::RawBytesUnchecked));
    buffer
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
