use ark_poly::polynomial::univariate::DensePolynomial;
use ark_poly_commit::marlin_pc::MarlinKZG10;
use ark_relations::r1cs::SynthesisError;
use ark_serialize::CanonicalDeserialize;
use ark_snark::SNARK;
use ark_std::{
    rand::{prelude::StdRng, SeedableRng},
    vec::Vec,
};
use blake2::Blake2s;
use codec::{Decode, Encode};
use frame_support::{
    log::{error, info},
    PalletError,
};
use scale_info::TypeInfo;

/// Possible errors from the verification process.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Decode, Encode, TypeInfo, PalletError)]
pub enum VerificationError {
    /// The verifying key was malformed.
    ///
    /// May occur only for some non-universal system.
    MalformedVerifyingKey,
    /// There was an error in the underlying holographic IOP. For details, consult your logs.
    ///
    /// May occur only for some universal system.
    AHPError,
    /// There was an error in the underlying polynomial commitment. For details, consult your logs.
    ///
    /// May occur only for some universal system.
    PolynomialCommitmentError,
    /// Unexpected error has occurred. Check your logs.
    UnexpectedError,
}

impl From<SynthesisError> for VerificationError {
    fn from(syn_err: SynthesisError) -> Self {
        match syn_err {
            SynthesisError::MalformedVerifyingKey => VerificationError::MalformedVerifyingKey,
            _ => {
                error!("Unexpected SynthesisError variant: {syn_err}");
                VerificationError::UnexpectedError
            }
        }
    }
}

impl From<ark_marlin::Error<ark_poly_commit::Error>> for VerificationError {
    fn from(err: ark_marlin::Error<ark_poly_commit::Error>) -> Self {
        match err {
            ark_marlin::Error::AHPError(err) => {
                info!("Encountered AHP error: {err:?}");
                VerificationError::AHPError
            }
            ark_marlin::Error::PolynomialCommitmentError(err) => {
                info!("Encountered polynomial commitment error: {err:?}");
                VerificationError::PolynomialCommitmentError
            }
            _ => {
                error!("Unexpected Marlin error variant: {err:?}");
                VerificationError::UnexpectedError
            }
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Decode, Encode, TypeInfo)]
pub enum ProvingSystem {
    Groth16,
    Gm17,
    Marlin,
}

pub(super) trait VerifyingSystem {
    type CircuitField: CanonicalDeserialize;
    type Proof: CanonicalDeserialize;
    type VerifyingKey: CanonicalDeserialize;

    fn verify(
        key: &Self::VerifyingKey,
        input: &[Self::CircuitField],
        proof: &Self::Proof,
        randomness: &[u8],
    ) -> Result<bool, VerificationError>;
}

/// Common pairing engine.
pub type DefaultPairingEngine = ark_bls12_381::Bls12_381;
/// Common scalar field.
pub type DefaultCircuitField = ark_bls12_381::Fr;

pub(super) struct Groth16;
impl VerifyingSystem for Groth16 {
    type CircuitField = DefaultCircuitField;
    type Proof = ark_groth16::Proof<DefaultPairingEngine>;
    type VerifyingKey = ark_groth16::VerifyingKey<DefaultPairingEngine>;

    fn verify(
        key: &Self::VerifyingKey,
        input: &[Self::CircuitField],
        proof: &Self::Proof,
        _: &[u8],
    ) -> Result<bool, VerificationError> {
        ark_groth16::Groth16::verify(key, input, proof).map_err(Into::into)
    }
}

pub(super) struct Gm17;
impl VerifyingSystem for Gm17 {
    type CircuitField = DefaultCircuitField;
    type Proof = ark_gm17::Proof<DefaultPairingEngine>;
    type VerifyingKey = ark_gm17::VerifyingKey<DefaultPairingEngine>;

    fn verify(
        key: &Self::VerifyingKey,
        input: &[Self::CircuitField],
        proof: &Self::Proof,
        _: &[u8],
    ) -> Result<bool, VerificationError> {
        ark_gm17::GM17::verify(key, input, proof).map_err(Into::into)
    }
}

type MarlinPolynomialCommitment =
    MarlinKZG10<DefaultPairingEngine, DensePolynomial<DefaultCircuitField>>;

pub(super) struct Marlin;
impl VerifyingSystem for Marlin {
    type CircuitField = DefaultCircuitField;
    type Proof = ark_marlin::Proof<DefaultCircuitField, MarlinPolynomialCommitment>;
    type VerifyingKey =
        ark_marlin::IndexVerifierKey<DefaultCircuitField, MarlinPolynomialCommitment>;

    fn verify(
        key: &Self::VerifyingKey,
        input: &[Self::CircuitField],
        proof: &Self::Proof,
        randomness: &[u8],
    ) -> Result<bool, VerificationError> {
        let seed = randomness
            .iter()
            .cloned()
            .cycle()
            .take(32)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap_or_default();
        let mut rng = StdRng::from_seed(seed);
        ark_marlin::Marlin::<_, _, Blake2s>::verify(key, input, proof, &mut rng).map_err(Into::into)
    }
}
