use ark_r1cs_std::alloc::AllocVar;
use ark_relations::ns;
use liminal_ark_relation_macro::snark_relation;

use crate::{BackendNote, FrontendNote};

/// 'Deposit' relation for the Shielder application.
///
/// It expresses the fact that `note` is a prefix of the result of hashing together `token_id`,
/// `token_amount`, `trapdoor` and `nullifier`.
#[snark_relation]
mod relation {
    use crate::{
        environment::FpVar,
        shielder::{
            convert_hash,
            note::check_note,
            types::{
                BackendNullifier, BackendTokenAmount, BackendTokenId, BackendTrapdoor,
                FrontendNullifier, FrontendTokenAmount, FrontendTokenId, FrontendTrapdoor,
            },
        },
    };

    #[relation_object_definition]
    struct DepositRelation {
        #[public_input(frontend_type = "FrontendNote", parse_with = "convert_hash")]
        pub note: BackendNote,
        #[public_input(frontend_type = "FrontendTokenId")]
        pub token_id: BackendTokenId,
        #[public_input(frontend_type = "FrontendTokenAmount")]
        pub token_amount: BackendTokenAmount,

        #[private_input(frontend_type = "FrontendTrapdoor")]
        pub trapdoor: BackendTrapdoor,
        #[private_input(frontend_type = "FrontendNullifier")]
        pub nullifier: BackendNullifier,
    }

    #[circuit_definition]
    fn generate_constraints() {
        let note = FpVar::new_input(ns!(cs, "note"), || self.note())?;
        let token_id = FpVar::new_input(ns!(cs, "token id"), || self.token_id())?;
        let token_amount = FpVar::new_input(ns!(cs, "token amount"), || self.token_amount())?;

        let trapdoor = FpVar::new_witness(ns!(cs, "trapdoor"), || self.trapdoor())?;
        let nullifier = FpVar::new_witness(ns!(cs, "nullifier"), || self.nullifier())?;

        check_note(&token_id, &token_amount, &trapdoor, &nullifier, &note)
    }
}

#[cfg(test)]
mod tests {
    use ark_bls12_381::Bls12_381;
    use ark_groth16::Groth16;
    use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem};
    use ark_snark::SNARK;

    use super::{
        DepositRelationWithFullInput, DepositRelationWithPublicInput, DepositRelationWithoutInput,
    };
    use crate::{
        shielder::note::compute_note, FrontendNullifier, FrontendTokenAmount, FrontendTokenId,
        FrontendTrapdoor,
    };

    fn get_circuit_with_full_input() -> DepositRelationWithFullInput {
        let token_id: FrontendTokenId = 1;
        let token_amount: FrontendTokenAmount = 10;
        let trapdoor: FrontendTrapdoor = 17;
        let nullifier: FrontendNullifier = 19;
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
