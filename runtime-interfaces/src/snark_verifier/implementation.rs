use halo2_proofs::{
    plonk::{verify_proof, Error, VerifyingKey},
    poly::kzg::{commitment::ParamsVerifierKZG, multiopen::VerifierGWC, strategy::SingleStrategy},
    standard_plonk::StandardPlonk,
    transcript::{Blake2bRead, TranscriptReadBuffer},
    SerdeFormat,
};

use crate::snark_verifier::VerifierError;

/// Elliptic curve used in the supported SNARKs.
pub type Curve = halo2_proofs::halo2curves::bn256::Bn256;
/// Affine representation of the elliptic curve used in the supported SNARKs.
pub type G1Affine = halo2_proofs::halo2curves::bn256::G1Affine;
/// Scalar field of the supported SNARKs.
pub type Fr = halo2_proofs::halo2curves::bn256::Fr;

pub fn do_verify(
    proof: &[u8],
    public_input: &[u8],
    verifying_key: &[u8],
) -> Result<(), VerifierError> {
    let instances = deserialize_public_input(public_input)?;
    let verifying_key = deserialize_verifying_key(verifying_key)?;
    let params = ParamsVerifierKZG::<Curve>::mock(crate::snark_verifier::CIRCUIT_MAX_K);

    verify_proof::<_, VerifierGWC<_>, _, _, _>(
        &params,
        &verifying_key,
        SingleStrategy::new(&params),
        &[&[&instances]],
        &mut Blake2bRead::init(&proof[..]),
    )
    .map_err(|err| match err {
        Error::ConstraintSystemFailure => VerifierError::IncorrectProof,
        _ => {
            log::debug!("Failed to verify a proof: {err:?}");
            VerifierError::VerificationFailed
        }
    })
}

fn deserialize_public_input(raw: &[u8]) -> Result<Vec<Fr>, VerifierError> {
    raw.chunks(32)
        .map(|bytes| {
            let bytes = bytes.try_into().map_err(|_| {
                log::debug!("Public input length is not multiple of 32");
                VerifierError::DeserializingPublicInputFailed
            })?;
            Option::from(Fr::from_bytes(bytes)).ok_or(VerifierError::DeserializingPublicInputFailed)
        })
        .collect::<Result<Vec<_>, _>>()
}

fn deserialize_verifying_key(key: &[u8]) -> Result<VerifyingKey<G1Affine>, VerifierError> {
    // We use `SerdeFormat::RawBytesUnchecked` here for performance reasons.
    VerifyingKey::from_bytes::<StandardPlonk>(key, SerdeFormat::RawBytesUnchecked).map_err(|err| {
        log::debug!("Failed to deserialize verification key: {err:?}");
        VerifierError::DeserializingVerificationKeyFailed
    })
}
