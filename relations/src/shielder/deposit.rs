use std::marker::PhantomData;

use ark_ff::BigInteger256;
use ark_r1cs_std::alloc::AllocVar;
use ark_relations::{
    ns,
    r1cs::{
        ConstraintSynthesizer, ConstraintSystemRef, SynthesisError,
        SynthesisError::AssignmentMissing,
    },
};

use super::{
    note::check_note,
    types::{
        BackendNote, BackendNullifier, BackendTokenAmount, BackendTokenId, BackendTrapdoor, FpVar,
        FrontendNote, FrontendNullifier, FrontendTokenAmount, FrontendTokenId, FrontendTrapdoor,
    },
};
use crate::{
    environment::CircuitField,
    relation::{
        state::{FullInput, NoInput, OnlyPublicInput, State, WithPublicInput},
        GetPublicInput,
    },
};

/// 'Deposit' relation for the Shielder application.
///
/// It expresses the fact that `note` is a prefix of the result of tangling together `token_id`,
/// `token_amount`, `trapdoor` and `nullifier`.
#[derive(Clone)]
pub struct DepositRelation<S: State> {
    // Public inputs
    pub note: Option<BackendNote>,
    pub token_id: Option<BackendTokenId>,
    pub token_amount: Option<BackendTokenAmount>,

    // Private inputs
    pub trapdoor: Option<BackendTrapdoor>,
    pub nullifier: Option<BackendNullifier>,

    _phantom: PhantomData<S>,
}

impl DepositRelation<NoInput> {
    pub fn without_input() -> Self {
        DepositRelation {
            note: None,
            token_id: None,
            token_amount: None,
            trapdoor: None,
            nullifier: None,
            _phantom: PhantomData,
        }
    }
}

impl DepositRelation<OnlyPublicInput> {
    pub fn with_public_input(
        note: FrontendNote,
        token_id: FrontendTokenId,
        token_amount: FrontendTokenAmount,
    ) -> Self {
        DepositRelation {
            note: Some(BackendNote::from(BigInteger256::new(note))),
            token_id: Some(BackendTokenId::from(token_id)),
            token_amount: Some(BackendTokenAmount::from(token_amount)),
            trapdoor: None,
            nullifier: None,
            _phantom: PhantomData,
        }
    }
}

impl DepositRelation<FullInput> {
    pub fn with_full_input(
        note: FrontendNote,
        token_id: FrontendTokenId,
        token_amount: FrontendTokenAmount,
        trapdoor: FrontendTrapdoor,
        nullifier: FrontendNullifier,
    ) -> Self {
        DepositRelation {
            note: Some(BackendNote::from(BigInteger256::new(note))),
            token_id: Some(BackendTokenId::from(token_id)),
            token_amount: Some(BackendTokenAmount::from(token_amount)),
            trapdoor: Some(BackendTrapdoor::from(trapdoor)),
            nullifier: Some(BackendNullifier::from(nullifier)),
            _phantom: PhantomData,
        }
    }
}

impl<S: State> ConstraintSynthesizer<CircuitField> for DepositRelation<S> {
    fn generate_constraints(
        self,
        cs: ConstraintSystemRef<CircuitField>,
    ) -> Result<(), SynthesisError> {
        let note = FpVar::new_input(ns!(cs, "note"), || self.note.ok_or(AssignmentMissing))?;
        let token_id = FpVar::new_input(ns!(cs, "token id"), || {
            self.token_id.ok_or(AssignmentMissing)
        })?;
        let token_amount = FpVar::new_input(ns!(cs, "token amount"), || {
            self.token_amount.ok_or(AssignmentMissing)
        })?;

        let trapdoor = FpVar::new_witness(ns!(cs, "trapdoor"), || {
            self.trapdoor.ok_or(AssignmentMissing)
        })?;
        let nullifier = FpVar::new_witness(ns!(cs, "nullifier"), || {
            self.nullifier.ok_or(AssignmentMissing)
        })?;

        check_note(&token_id, &token_amount, &trapdoor, &nullifier, &note)
    }
}

impl<S: WithPublicInput> GetPublicInput<CircuitField> for DepositRelation<S> {
    fn public_input(&self) -> Vec<CircuitField> {
        vec![
            self.note.unwrap(),
            self.token_id.unwrap(),
            self.token_amount.unwrap(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use ark_bls12_381::Bls12_381;
    use ark_groth16::Groth16;
    use ark_relations::r1cs::ConstraintSystem;
    use ark_snark::SNARK;

    use super::*;
    use crate::shielder::note::compute_note;

    fn get_circuit_with_full_input() -> DepositRelation<FullInput> {
        let token_id: FrontendTokenId = 1;
        let token_amount: FrontendTokenAmount = 10;
        let trapdoor: FrontendTrapdoor = 17;
        let nullifier: FrontendNullifier = 19;
        let note = compute_note(token_id, token_amount, trapdoor, nullifier);

        DepositRelation::with_full_input(note, token_id, token_amount, trapdoor, nullifier)
    }

    #[test]
    fn deposit_constraints_correctness() {
        let circuit = get_circuit_with_full_input();

        let cs = ConstraintSystem::new_ref();
        circuit.generate_constraints(cs.clone()).unwrap();

        let is_satisfied = cs.is_satisfied().unwrap();
        if !is_satisfied {
            println!("{:?}", cs.which_is_unsatisfied());
        }

        assert!(is_satisfied);
    }

    #[test]
    fn deposit_proving_procedure() {
        let circuit_wo_input = DepositRelation::without_input();

        let mut rng = ark_std::test_rng();
        let (pk, vk) =
            Groth16::<Bls12_381>::circuit_specific_setup(circuit_wo_input, &mut rng).unwrap();

        let circuit = get_circuit_with_full_input();
        let input = circuit.public_input();

        let proof = Groth16::prove(&pk, circuit, &mut rng).unwrap();
        let valid_proof = Groth16::verify(&vk, &input, &proof).unwrap();
        assert!(valid_proof);
    }
}
