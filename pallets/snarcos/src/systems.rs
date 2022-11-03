use ark_poly::polynomial::univariate::DensePolynomial;
use ark_poly_commit::marlin_pc::MarlinKZG10;
use ark_serialize::CanonicalDeserialize;
use ark_snark::SNARK;
use ark_std::rand::{prelude::StdRng, SeedableRng};
use blake2::Blake2s;
use codec::{Decode, Encode};
use scale_info::TypeInfo;

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
    ) -> Result<bool, ()>;
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
    ) -> Result<bool, ()> {
        ark_groth16::Groth16::verify(key, input, proof).map_err(|_| ())
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
    ) -> Result<bool, ()> {
        ark_gm17::GM17::verify(key, input, proof).map_err(|_| ())
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
    ) -> Result<bool, ()> {
        let mut rng = StdRng::from_seed([0u8; 32]);
        ark_marlin::Marlin::<_, _, Blake2s>::verify(key, input, proof, &mut rng).map_err(|_| ())
    }
}
