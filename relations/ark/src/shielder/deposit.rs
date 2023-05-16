use liminal_ark_relation_macro::snark_relation;

use super::types::{BackendNote, FrontendNote};

/// 'Deposit' relation for the Shielder application.
///
/// It expresses the fact that `note` is a prefix of the result of hashing together `token_id`,
/// `token_amount`, `trapdoor` and `nullifier`.
#[snark_relation]
mod relation {
    #[cfg(feature = "circuit")]
    use {
        crate::shielder::note_var::NoteVarBuilder,
        ark_r1cs_std::alloc::AllocationMode::{Input, Witness},
    };

    use crate::shielder::{
        convert_hash,
        types::{
            BackendNullifier, BackendTokenAmount, BackendTokenId, BackendTrapdoor,
            FrontendNullifier, FrontendTokenAmount, FrontendTokenId, FrontendTrapdoor,
        },
    };

    #[relation_object_definition]
    #[derive(Clone, Debug)]
    struct DepositRelation {
        #[public_input(frontend_type = "FrontendNote", parse_with = "convert_hash")]
        pub note: BackendNote,
        #[public_input(frontend_type = "FrontendTokenId")]
        pub token_id: BackendTokenId,
        #[public_input(frontend_type = "FrontendTokenAmount")]
        pub token_amount: BackendTokenAmount,

        #[private_input(frontend_type = "FrontendTrapdoor", parse_with = "convert_hash")]
        pub trapdoor: BackendTrapdoor,
        #[private_input(frontend_type = "FrontendNullifier", parse_with = "convert_hash")]
        pub nullifier: BackendNullifier,
    }

    #[cfg(feature = "circuit")]
    #[circuit_definition]
    fn generate_constraints() {
        let _note = NoteVarBuilder::new(cs)
            .with_note(self.note(), Input)?
            .with_token_id(self.token_id(), Input)?
            .with_token_amount(self.token_amount(), Input)?
            .with_trapdoor(self.trapdoor(), Witness)?
            .with_nullifier(self.nullifier(), Witness)?
            .build()?;
        Ok(())
    }
}

#[cfg(all(test, feature = "circuit"))]
mod tests {
    use ark_bls12_381::Bls12_381;
    use ark_groth16::Groth16;
    use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem};
    use ark_snark::SNARK;

    use super::{
        DepositRelationWithFullInput, DepositRelationWithPublicInput, DepositRelationWithoutInput,
    };
    use crate::shielder::{
        note::compute_note,
        types::{FrontendNullifier, FrontendTokenAmount, FrontendTokenId, FrontendTrapdoor},
    };

    fn get_circuit_with_full_input() -> DepositRelationWithFullInput {
        let token_id: FrontendTokenId = 1;
        let token_amount: FrontendTokenAmount = 100_000_000_000_000_000_000;
        let trapdoor: FrontendTrapdoor = [17; 4];
        let nullifier: FrontendNullifier = [19; 4];
        let note = compute_note(token_id, token_amount, trapdoor, nullifier);

        DepositRelationWithFullInput::new(note, token_id, token_amount, trapdoor, nullifier)
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
        let circuit_withouth_input = DepositRelationWithoutInput::new();

        let mut rng = ark_std::test_rng();
        let (pk, vk) =
            Groth16::<Bls12_381>::circuit_specific_setup(circuit_withouth_input, &mut rng).unwrap();

        let circuit = get_circuit_with_full_input();
        let proof = Groth16::prove(&pk, circuit, &mut rng).unwrap();

        let circuit: DepositRelationWithPublicInput = get_circuit_with_full_input().into();
        let input = circuit.serialize_public_input();
        let valid_proof = Groth16::verify(&vk, &input, &proof).unwrap();
        assert!(valid_proof);
    }
}
