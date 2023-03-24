use ark_std::vec::Vec;
#[cfg(feature = "circuit")]
use {
    ark_poly::univariate::DensePolynomial,
    ark_poly_commit::marlin_pc::MarlinKZG10,
    ark_relations::r1cs::ConstraintSynthesizer,
    ark_serialize::{CanonicalDeserialize, CanonicalSerialize},
    ark_snark::SNARK,
    ark_std::rand::{rngs::StdRng, SeedableRng},
    blake2::Blake2s,
};

// For now, we can settle with these types.
/// Common pairing engine.
pub type PairingEngine = ark_bls12_381::Bls12_381;
/// Common scalar field.
pub type CircuitField = ark_bls12_381::Fr;
#[cfg(feature = "circuit")]
/// Variable in the Fr field
pub type FpVar = ark_r1cs_std::fields::fp::FpVar<CircuitField>;

// Systems with hardcoded parameters.
#[cfg(feature = "circuit")]
pub type Groth16 = ark_groth16::Groth16<PairingEngine>;
#[cfg(feature = "circuit")]
pub type GM17 = ark_gm17::GM17<PairingEngine>;
#[cfg(feature = "circuit")]
pub type MarlinPolynomialCommitment = MarlinKZG10<PairingEngine, DensePolynomial<CircuitField>>;
#[cfg(feature = "circuit")]
pub type Marlin = ark_marlin::Marlin<CircuitField, MarlinPolynomialCommitment, Blake2s>;

/// Serialized keys.
pub struct RawKeys {
    pub pk: Vec<u8>,
    pub vk: Vec<u8>,
}

#[cfg(feature = "circuit")]
pub enum Error {
    UniversalSystemVerificationError,
    NonUniversalSystemVerificationError,
}

/// Common API for every proving system.
#[cfg(feature = "circuit")]
pub trait ProvingSystem {
    type Proof: CanonicalSerialize + CanonicalDeserialize;
    type ProvingKey: CanonicalSerialize + CanonicalDeserialize;
    type VerifyingKey: CanonicalSerialize + CanonicalDeserialize;

    /// Generates proof for `circuit` using proving key `pk`
    fn prove<C: ConstraintSynthesizer<CircuitField>>(
        pk: &Self::ProvingKey,
        circuit: C,
    ) -> Self::Proof;

    // parametrize over Proving System
    fn verify(
        vk: &Self::VerifyingKey,
        proof: &Self::Proof,
        public_input: Vec<CircuitField>,
    ) -> Result<bool, Error>;
}

/// Common API for every universal proving system.
#[cfg(feature = "circuit")]
pub trait UniversalSystem: ProvingSystem {
    type Srs: CanonicalSerialize + CanonicalDeserialize;

    /// Generates SRS.
    fn generate_srs(num_constraints: usize, num_variables: usize, degree: usize) -> Self::Srs;

    /// Generates proving and verifying key for `circuit` using `srs`.
    fn generate_keys<C: ConstraintSynthesizer<CircuitField>>(
        circuit: C,
        srs: &Self::Srs,
    ) -> (Self::ProvingKey, Self::VerifyingKey);
}

/// Common API for every non universal proving system.
#[cfg(feature = "circuit")]
pub trait NonUniversalSystem: ProvingSystem {
    /// Generates proving and verifying key for `circuit`.
    fn generate_keys<C: ConstraintSynthesizer<CircuitField>>(
        circuit: C,
    ) -> (Self::ProvingKey, Self::VerifyingKey);
}

#[cfg(feature = "circuit")]
fn dummy_rng() -> StdRng {
    StdRng::from_seed([0u8; 32])
}

// Unfortunately, Groth16, GM17 and Marlin don't have any common supertrait, and therefore,
// we cannot provide any blanket implementation without running into damned `upstream crates may
// add a new impl of trait` error (see https://github.com/rust-lang/rfcs/issues/2758).
// Tfu. Disgusting.

/// This macro takes a type `system` as the only argument and provides `ProvingSystem` and
/// `NonUniversalSystem` implementations for it.
///
/// `system` should implement `SNARK<CircuitField>` trait.  
#[cfg(feature = "circuit")]
macro_rules! impl_non_universal_system_for_snark {
    ($system:ty) => {
        impl ProvingSystem for $system {
            type Proof = <$system as SNARK<CircuitField>>::Proof;
            type ProvingKey = <$system as SNARK<CircuitField>>::ProvingKey;
            type VerifyingKey = <$system as SNARK<CircuitField>>::VerifyingKey;

            fn prove<C: ConstraintSynthesizer<CircuitField>>(
                pk: &Self::ProvingKey,
                circuit: C,
            ) -> Self::Proof {
                let mut rng = dummy_rng();
                <$system as SNARK<CircuitField>>::prove(pk, circuit, &mut rng)
                    .expect("Failed to generate proof")
            }

            fn verify(
                vk: &Self::VerifyingKey,
                proof: &Self::Proof,
                public_input: Vec<CircuitField>,
            ) -> Result<bool, Error> {
                <$system as SNARK<CircuitField>>::verify(vk, &*public_input, proof)
                    .map_err(|_why| Error::NonUniversalSystemVerificationError)
            }
        }

        impl NonUniversalSystem for $system {
            fn generate_keys<C: ConstraintSynthesizer<CircuitField>>(
                circuit: C,
            ) -> (Self::ProvingKey, Self::VerifyingKey) {
                let mut rng = dummy_rng();
                <$system as SNARK<CircuitField>>::circuit_specific_setup(circuit, &mut rng)
                    .expect("Failed to generate keys")
            }
        }
    };
}

#[cfg(feature = "circuit")]
impl_non_universal_system_for_snark!(Groth16);
#[cfg(feature = "circuit")]
impl_non_universal_system_for_snark!(GM17);

#[cfg(feature = "circuit")]
impl ProvingSystem for Marlin {
    type Proof = ark_marlin::Proof<CircuitField, MarlinPolynomialCommitment>;
    type ProvingKey = ark_marlin::IndexProverKey<CircuitField, MarlinPolynomialCommitment>;
    type VerifyingKey = ark_marlin::IndexVerifierKey<CircuitField, MarlinPolynomialCommitment>;

    fn prove<C: ConstraintSynthesizer<CircuitField>>(
        pk: &Self::ProvingKey,
        circuit: C,
    ) -> Self::Proof {
        let mut rng = dummy_rng();
        Marlin::prove(pk, circuit, &mut rng).expect("Failed to generate proof")
    }

    fn verify(
        vk: &Self::VerifyingKey,
        proof: &Self::Proof,
        public_input: Vec<CircuitField>,
    ) -> Result<bool, Error> {
        let mut rng = dummy_rng();
        Marlin::verify(vk, public_input.as_slice(), proof, &mut rng)
            .map_err(|_why| Error::UniversalSystemVerificationError)
    }
}

#[cfg(feature = "circuit")]
impl UniversalSystem for Marlin {
    type Srs = ark_marlin::UniversalSRS<CircuitField, MarlinPolynomialCommitment>;

    fn generate_srs(num_constraints: usize, num_variables: usize, degree: usize) -> Self::Srs {
        let mut rng = dummy_rng();
        Marlin::universal_setup(num_constraints, num_variables, degree, &mut rng)
            .expect("Failed to generate SRS")
    }

    fn generate_keys<C: ConstraintSynthesizer<CircuitField>>(
        circuit: C,
        srs: &Self::Srs,
    ) -> (Self::ProvingKey, Self::VerifyingKey) {
        Marlin::index(srs, circuit).expect(
            "Failed to generate keys from SRS (it might be the case, that the circuit is \
                larger than the SRS allows).",
        )
    }
}
