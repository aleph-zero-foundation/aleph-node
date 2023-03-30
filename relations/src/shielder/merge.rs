use liminal_ark_relation_macro::snark_relation;

/// It expresses the facts that:
///  - `first_old_note` is the result of hashing together the `token_id`,
///    `first_old_token_amount`, `first_old_trapdoor` and `first_old_nullifier`,
///  - `second_old_note` is the result of hashing together the `token_id`,
///    `second_old_token_amount`, `second_old_trapdoor` and `second_old_nullifier`,
///  - `new_note` is the result of hashing together the `token_id`, `new_token_amount`,
///    `new_trapdoor` and `new_nullifier`,
///  - `new_token_amount = token_amount + old_token_amount`
///  - `first_merkle_path` is a valid Merkle proof for `first_old_note` being present
///    at `first_leaf_index` in some Merkle tree with `merkle_root` hash in the root
///  - `second_merkle_path` is a valid Merkle proof for `second_old_note` being present
///    at `second_leaf_index` in some Merkle tree with `merkle_root` hash in the root
/// Additionally, the relation has one constant input, `max_path_len` which specifies upper bound
/// for the length of the merkle path (which is ~the height of the tree, ±1).
#[snark_relation]
mod relation {
    #[cfg(feature = "circuit")]
    use {
        crate::shielder::{
            check_merkle_proof, note_var::NoteVarBuilder, path_shape_var::PathShapeVar,
        },
        ark_r1cs_std::{
            alloc::{
                AllocVar,
                AllocationMode::{Input, Witness},
            },
            eq::EqGadget,
            fields::fp::FpVar,
        },
        ark_relations::ns,
        core::ops::Add,
    };

    use crate::shielder::{
        convert_hash, convert_vec,
        types::{
            BackendLeafIndex, BackendMerklePath, BackendMerkleRoot, BackendNote, BackendNullifier,
            BackendTokenAmount, BackendTokenId, BackendTrapdoor, FrontendLeafIndex,
            FrontendMerklePath, FrontendMerkleRoot, FrontendNote, FrontendNullifier,
            FrontendTokenAmount, FrontendTokenId, FrontendTrapdoor,
        },
    };

    #[relation_object_definition]
    #[derive(Clone, Debug)]
    struct MergeRelation {
        #[constant]
        pub max_path_len: u8,

        // Public inputs
        #[public_input(frontend_type = "FrontendTokenId")]
        pub token_id: BackendTokenId,
        #[public_input(frontend_type = "FrontendNullifier", parse_with = "convert_hash")]
        pub first_old_nullifier: BackendNullifier,
        #[public_input(frontend_type = "FrontendNullifier", parse_with = "convert_hash")]
        pub second_old_nullifier: BackendNullifier,
        #[public_input(frontend_type = "FrontendNote", parse_with = "convert_hash")]
        pub new_note: BackendNote,
        #[public_input(frontend_type = "FrontendMerkleRoot", parse_with = "convert_hash")]
        pub merkle_root: BackendMerkleRoot,

        // Private inputs.
        #[private_input(frontend_type = "FrontendTrapdoor", parse_with = "convert_hash")]
        pub first_old_trapdoor: BackendTrapdoor,
        #[private_input(frontend_type = "FrontendTrapdoor", parse_with = "convert_hash")]
        pub second_old_trapdoor: BackendTrapdoor,
        #[private_input(frontend_type = "FrontendTrapdoor", parse_with = "convert_hash")]
        pub new_trapdoor: BackendTrapdoor,
        #[private_input(frontend_type = "FrontendNullifier", parse_with = "convert_hash")]
        pub new_nullifier: BackendNullifier,
        #[private_input(frontend_type = "FrontendMerklePath", parse_with = "convert_vec")]
        pub first_merkle_path: BackendMerklePath,
        #[private_input(frontend_type = "FrontendMerklePath", parse_with = "convert_vec")]
        pub second_merkle_path: BackendMerklePath,
        #[private_input(frontend_type = "FrontendLeafIndex")]
        pub first_leaf_index: BackendLeafIndex,
        #[private_input(frontend_type = "FrontendLeafIndex")]
        pub second_leaf_index: BackendLeafIndex,
        #[private_input(frontend_type = "FrontendNote", parse_with = "convert_hash")]
        pub first_old_note: BackendNote,
        #[private_input(frontend_type = "FrontendNote", parse_with = "convert_hash")]
        pub second_old_note: BackendNote,
        #[private_input(frontend_type = "FrontendTokenAmount")]
        pub first_old_token_amount: BackendTokenAmount,
        #[private_input(frontend_type = "FrontendTokenAmount")]
        pub second_old_token_amount: BackendTokenAmount,
        #[private_input(frontend_type = "FrontendTokenAmount")]
        pub new_token_amount: BackendTokenAmount,
    }

    #[cfg(feature = "circuit")]
    #[circuit_definition]
    fn generate_constraints() {
        //------------------------------
        // Check first old note arguments.
        //------------------------------
        let first_old_note = NoteVarBuilder::new(cs.clone())
            .with_token_id(self.token_id(), Input)?
            .with_token_amount(self.first_old_token_amount(), Witness)?
            .with_trapdoor(self.first_old_trapdoor(), Witness)?
            .with_nullifier(self.first_old_nullifier(), Input)?
            .with_note(self.first_old_note(), Witness)?
            .build()?;

        //------------------------------
        // Check second old note arguments.
        //------------------------------
        let second_old_note = NoteVarBuilder::new(cs.clone())
            .with_token_id_var(first_old_note.token_id.clone())
            .with_token_amount(self.second_old_token_amount(), Witness)?
            .with_trapdoor(self.second_old_trapdoor(), Witness)?
            .with_nullifier(self.second_old_nullifier(), Input)?
            .with_note(self.second_old_note(), Witness)?
            .build()?;

        //------------------------------
        // Check new note arguments.
        //------------------------------
        let new_note = NoteVarBuilder::new(cs.clone())
            .with_token_id_var(first_old_note.token_id.clone())
            .with_token_amount(self.new_token_amount(), Witness)?
            .with_trapdoor(self.new_trapdoor(), Witness)?
            .with_nullifier(self.new_nullifier(), Witness)?
            .with_note(self.new_note(), Input)?
            .build()?;

        //----------------------------------
        // Check token value soundness.
        //----------------------------------
        let token_sum = first_old_note
            .token_amount
            .add(second_old_note.token_amount)?;
        token_sum.enforce_equal(&new_note.token_amount)?;

        //------------------------
        // Check first merkle proof.
        //------------------------
        let merkle_root = FpVar::new_input(ns!(cs, "merkle root"), || self.merkle_root())?;
        let first_path_shape = PathShapeVar::new_witness(ns!(cs, "first path shape"), || {
            Ok((*self.max_path_len(), self.first_leaf_index().cloned()))
        })?;

        check_merkle_proof(
            merkle_root.clone(),
            first_path_shape,
            first_old_note.note,
            self.first_merkle_path().cloned().unwrap_or_default(),
            *self.max_path_len(),
            cs.clone(),
        )?;

        //------------------------
        // Check second merkle proof.
        //------------------------
        let second_path_shape = PathShapeVar::new_witness(ns!(cs, "second path shape"), || {
            Ok((*self.max_path_len(), self.second_leaf_index().cloned()))
        })?;

        check_merkle_proof(
            merkle_root,
            second_path_shape,
            second_old_note.note,
            self.second_merkle_path().cloned().unwrap_or_default(),
            *self.max_path_len(),
            cs,
        )
    }
}

#[cfg(all(test, feature = "circuit"))]
mod tests {
    use ark_bls12_381::Bls12_381;
    use ark_groth16::Groth16;
    use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem};
    use ark_snark::SNARK;

    use super::*;
    use crate::shielder::{
        convert_hash,
        note::{compute_note, compute_parent_hash},
        types::FrontendNote,
    };

    const MAX_PATH_LEN: u8 = 4;
    const TOKEN_ID: FrontendTokenId = 1;

    const FIRST_OLD_TRAPDOOR: FrontendTrapdoor = [17; 4];
    const FIRST_OLD_NULLIFIER: FrontendNullifier = [19; 4];
    const FIRST_OLD_TOKEN_AMOUNT: FrontendTokenAmount = 3;

    const SECOND_OLD_TRAPDOOR: FrontendTrapdoor = [23; 4];
    const SECOND_OLD_NULLIFIER: FrontendNullifier = [29; 4];
    const SECOND_OLD_TOKEN_AMOUNT: FrontendTokenAmount = 7;

    const NEW_TRAPDOOR: FrontendTrapdoor = [27; 4];
    const NEW_NULLIFIER: FrontendNullifier = [87; 4];
    const NEW_TOKEN_AMOUNT: FrontendTokenAmount = 10;

    const FIRST_LEAF_INDEX: u64 = 5;
    const SECOND_LEAF_INDEX: u64 = 6;

    fn get_circuit_with_full_input() -> MergeRelationWithFullInput {
        let first_old_note = compute_note(
            TOKEN_ID,
            FIRST_OLD_TOKEN_AMOUNT,
            FIRST_OLD_TRAPDOOR,
            FIRST_OLD_NULLIFIER,
        );
        let second_old_note = compute_note(
            TOKEN_ID,
            SECOND_OLD_TOKEN_AMOUNT,
            SECOND_OLD_TRAPDOOR,
            SECOND_OLD_NULLIFIER,
        );
        let new_note = compute_note(TOKEN_ID, NEW_TOKEN_AMOUNT, NEW_TRAPDOOR, NEW_NULLIFIER);

        //                                          merkle root
        //                placeholder                                        x
        //        1                       x                     x                       x
        //   2         3              x        x            x       x              x       x
        // 4  *5*  ^6^   7          x   x    x   x        x   x   x   x          x   x   x   x
        //
        // *first_old_note* | ^second_old_note^

        let zero_note = FrontendNote::default(); // x

        // First Merkle path setup.
        let first_sibling_note = compute_note(0, 1, [2; 4], [3; 4]); // 4
        let first_parent_note = compute_parent_hash(first_sibling_note, first_old_note); // 2

        // Second Merkle path setup.
        let second_sibling_note = compute_note(0, 1, [3; 4], [4; 4]); // 7
        let second_parent_note = compute_parent_hash(second_old_note, second_sibling_note); // 3

        // Merkle paths.
        let first_merkle_path = vec![first_sibling_note, second_parent_note];
        let second_merkle_path = vec![second_sibling_note, first_parent_note];

        // Common roots.
        let grandpa_root = compute_parent_hash(first_parent_note, second_parent_note); // 1
        let placeholder = compute_parent_hash(grandpa_root, zero_note);
        let merkle_root = compute_parent_hash(placeholder, zero_note);

        MergeRelationWithFullInput::new(
            MAX_PATH_LEN,
            TOKEN_ID,
            FIRST_OLD_NULLIFIER,
            SECOND_OLD_NULLIFIER,
            new_note,
            merkle_root,
            FIRST_OLD_TRAPDOOR,
            SECOND_OLD_TRAPDOOR,
            NEW_TRAPDOOR,
            NEW_NULLIFIER,
            first_merkle_path,
            second_merkle_path,
            FIRST_LEAF_INDEX,
            SECOND_LEAF_INDEX,
            first_old_note,
            second_old_note,
            FIRST_OLD_TOKEN_AMOUNT,
            SECOND_OLD_TOKEN_AMOUNT,
            NEW_TOKEN_AMOUNT,
        )
    }

    fn get_circuit_with_invalid_first_old_note() -> MergeRelationWithFullInput {
        let mut circuit = get_circuit_with_full_input();

        let first_old_note = compute_note(
            TOKEN_ID,
            FIRST_OLD_TOKEN_AMOUNT + 1,
            FIRST_OLD_TRAPDOOR,
            FIRST_OLD_NULLIFIER,
        );
        circuit.first_old_note = convert_hash(first_old_note);

        circuit
    }

    fn get_circuit_with_invalid_second_old_note() -> MergeRelationWithFullInput {
        let mut circuit = get_circuit_with_full_input();

        let second_old_note = compute_note(
            TOKEN_ID,
            SECOND_OLD_TOKEN_AMOUNT + 1,
            SECOND_OLD_TRAPDOOR,
            SECOND_OLD_NULLIFIER,
        );
        circuit.second_old_note = convert_hash(second_old_note);

        circuit
    }

    fn get_circuit_with_invalid_new_note() -> MergeRelationWithFullInput {
        let mut circuit = get_circuit_with_full_input();
        let new_note = compute_note(
            TOKEN_ID,
            NEW_TOKEN_AMOUNT,
            NEW_TRAPDOOR.map(|t| t + 1),
            NEW_NULLIFIER,
        );
        circuit.new_note = convert_hash(new_note);

        circuit
    }

    fn get_circuit_with_unsound_value() -> MergeRelationWithFullInput {
        let mut circuit = get_circuit_with_full_input();

        let new_note = compute_note(TOKEN_ID, NEW_TOKEN_AMOUNT + 1, NEW_TRAPDOOR, NEW_NULLIFIER);
        circuit.new_note = convert_hash(new_note);

        circuit
    }

    fn get_circuit_with_invalid_first_leaf_index() -> MergeRelationWithFullInput {
        let mut circuit = get_circuit_with_full_input();
        circuit.first_leaf_index = FIRST_LEAF_INDEX + 1;
        circuit
    }

    fn get_circuit_with_invalid_second_leaf_index() -> MergeRelationWithFullInput {
        let mut circuit = get_circuit_with_full_input();
        circuit.second_leaf_index = SECOND_LEAF_INDEX + 1;
        circuit
    }

    fn merge_constraints_correctness(circuit: MergeRelationWithFullInput) -> bool {
        let cs = ConstraintSystem::new_ref();
        circuit.generate_constraints(cs.clone()).unwrap();

        let is_satisfied = cs.is_satisfied().unwrap();
        if !is_satisfied {
            println!("{:?}", cs.which_is_unsatisfied());
        }

        is_satisfied
    }

    fn merge_proving_procedure(circuit_generator: fn() -> MergeRelationWithFullInput) {
        let circuit_withouth_input = MergeRelationWithoutInput::new(MAX_PATH_LEN);

        let mut rng = ark_std::test_rng();
        let (pk, vk) =
            Groth16::<Bls12_381>::circuit_specific_setup(circuit_withouth_input, &mut rng).unwrap();

        let proof = Groth16::prove(&pk, circuit_generator(), &mut rng).unwrap();

        let circuit: MergeRelationWithPublicInput = circuit_generator().into();
        let input = circuit.serialize_public_input();

        let valid_proof = Groth16::verify(&vk, &input, &proof).unwrap();
        assert!(valid_proof);
    }

    #[test]
    fn merge_constraints_valid_circuit() {
        let circuit = get_circuit_with_full_input();

        let constraints_correctness = merge_constraints_correctness(circuit);
        assert!(constraints_correctness);
    }

    #[test]
    fn merge_proving_procedure_valid_circuit() {
        merge_proving_procedure(get_circuit_with_full_input);
    }

    #[test]
    fn merge_constraints_invalid_first_old_note() {
        let invalid_circuit = get_circuit_with_invalid_first_old_note();

        let constraints_correctness = merge_constraints_correctness(invalid_circuit);
        assert!(!constraints_correctness);
    }

    #[test]
    fn merge_constraints_invalid_second_old_note() {
        let invalid_circuit = get_circuit_with_invalid_second_old_note();

        let constraints_correctness = merge_constraints_correctness(invalid_circuit);
        assert!(!constraints_correctness);
    }

    #[test]
    fn merge_constraints_invalid_new_note() {
        let invalid_circuit = get_circuit_with_invalid_new_note();

        let constraints_correctness = merge_constraints_correctness(invalid_circuit);
        assert!(!constraints_correctness);
    }

    #[test]
    fn merge_constraints_unsound_value() {
        let invalid_circuit = get_circuit_with_unsound_value();

        let constraints_correctness = merge_constraints_correctness(invalid_circuit);
        assert!(!constraints_correctness);
    }

    #[test]
    fn merge_constraints_invalid_first_leaf_index() {
        let invalid_circuit = get_circuit_with_invalid_first_leaf_index();

        let constraints_correctness = merge_constraints_correctness(invalid_circuit);
        assert!(!constraints_correctness);
    }

    #[test]
    fn merge_constraints_invalid_second_leaf_index() {
        let invalid_circuit = get_circuit_with_invalid_second_leaf_index();

        let constraints_correctness = merge_constraints_correctness(invalid_circuit);
        assert!(!constraints_correctness);
    }
}
