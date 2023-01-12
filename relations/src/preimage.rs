// This relation showcases how to use Poseidon in r1cs circuits
use ark_r1cs_std::{alloc::AllocVar, eq::EqGadget};
use ark_relations::{
    ns,
    r1cs::{
        ConstraintSynthesizer, ConstraintSystemRef, SynthesisError,
        SynthesisError::AssignmentMissing,
    },
};
use ark_std::{marker::PhantomData, vec, vec::Vec};
use poseidon::circuit;

use crate::{
    environment::FpVar,
    relation::state::{FullInput, NoInput, OnlyPublicInput, State, WithPublicInput},
    CircuitField, GetPublicInput,
};

/// Preimage relation : H(preimage)=hash
/// where:
/// - hash : public input
/// - preimage : private witness
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct PreimageRelation<S: State> {
    // private witness
    pub preimage: Option<CircuitField>,
    // public input
    pub hash: Option<CircuitField>,
    _phantom: PhantomData<S>,
}

impl PreimageRelation<NoInput> {
    pub fn without_input() -> Self {
        Self {
            hash: None,
            preimage: None,
            _phantom: PhantomData,
        }
    }
}

impl PreimageRelation<OnlyPublicInput> {
    pub fn with_public_input(hash: CircuitField) -> Self {
        Self {
            preimage: None,
            hash: Some(hash),
            _phantom: PhantomData,
        }
    }
}

impl PreimageRelation<FullInput> {
    pub fn with_full_input(preimage: CircuitField, hash: CircuitField) -> Self {
        Self {
            preimage: Some(preimage),
            hash: Some(hash),
            _phantom: PhantomData,
        }
    }
}

impl<S: State> ConstraintSynthesizer<CircuitField> for PreimageRelation<S> {
    fn generate_constraints(
        self,
        cs: ConstraintSystemRef<CircuitField>,
    ) -> Result<(), SynthesisError> {
        let preimage = FpVar::new_witness(ns!(cs, "preimage"), || {
            self.preimage.ok_or(AssignmentMissing)
        })?;
        let hash = FpVar::new_input(ns!(cs, "hash"), || self.hash.ok_or(AssignmentMissing))?;
        let hash_result = circuit::one_to_one_hash(cs, preimage)?;

        hash.enforce_equal(&hash_result)?;

        Ok(())
    }
}

impl<S: WithPublicInput> GetPublicInput<CircuitField> for PreimageRelation<S> {
    fn public_input(&self) -> Vec<CircuitField> {
        vec![self
            .hash
            .expect("Circuit should have public input assigned")]
    }
}

#[cfg(test)]
mod tests {
    use ark_bls12_381::Bls12_381;
    use ark_crypto_primitives::SNARK;
    use ark_groth16::Groth16;
    use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem};
    use poseidon::hash;

    use super::PreimageRelation;
    use crate::{CircuitField, GetPublicInput};

    #[test]
    fn preimage_constraints_correctness() {
        let preimage = CircuitField::from(17u64);
        let image = hash::one_to_one_hash(preimage);

        let circuit = PreimageRelation::with_full_input(preimage, image);

        let cs = ConstraintSystem::new_ref();
        circuit.generate_constraints(cs.clone()).unwrap();

        let is_satisfied = cs.is_satisfied().unwrap();
        if !is_satisfied {
            println!("{:?}", cs.which_is_unsatisfied());
        }

        assert!(is_satisfied);
    }

    #[test]
    fn unsatisfied_preimage_constraints() {
        let true_preimage = CircuitField::from(17u64);
        let fake_image = hash::one_to_one_hash(CircuitField::from(19u64));
        let circuit = PreimageRelation::with_full_input(true_preimage, fake_image);

        let cs = ConstraintSystem::new_ref();
        circuit.generate_constraints(cs.clone()).unwrap();

        let is_satisfied = cs.is_satisfied().unwrap();

        assert!(!is_satisfied);
    }

    #[test]
    fn preimage_proving_and_verifying() {
        let preimage = CircuitField::from(7u64);
        let image = hash::one_to_one_hash(preimage);

        let circuit = PreimageRelation::with_full_input(preimage, image);

        let mut rng = ark_std::test_rng();
        let (pk, vk) =
            Groth16::<Bls12_381>::circuit_specific_setup(circuit.clone(), &mut rng).unwrap();

        let input = circuit.public_input();

        let proof = Groth16::prove(&pk, circuit, &mut rng).unwrap();

        let is_valid = Groth16::verify(&vk, &input, &proof).unwrap();
        assert!(is_valid);
    }
}
