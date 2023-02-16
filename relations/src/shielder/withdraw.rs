use liminal_ark_relation_macro::snark_relation;

/// 'Withdraw' relation for the Shielder application.
///
/// It expresses the facts that:
///  - `new_note` is a prefix of the result of hashing together `token_id`, `whole_token_amount`,
///    `old_trapdoor` and `old_nullifier`,
///  - `old_note` is a prefix of the result of hashing together `token_id`, `new_token_amount`,
///    `new_trapdoor` and `new_nullifier`,
///  - `new_token_amount + token_amount_out = whole_token_amount`
///  - `merkle_path` is a valid Merkle proof for `old_note` being present at `leaf_index` in some
///    Merkle tree with `merkle_root` hash in the root
/// It also includes two artificial inputs `fee` and `recipient` just to strengthen the application
/// security by treating them as public inputs (and thus integral part of the SNARK).
/// Additionally, the relation has one constant input, `max_path_len` which specifies upper bound
/// for the length of the merkle path (which is ~the height of the tree, ±1).
#[snark_relation]
mod relation {
    use core::ops::Add;

    use ark_r1cs_std::{alloc::AllocVar, eq::EqGadget, fields::fp::FpVar};
    use ark_relations::ns;

    use crate::shielder::{
        check_merkle_proof,
        circuit_utils::PathShapeVar,
        convert_account, convert_hash, convert_vec,
        note::check_note,
        types::{
            BackendAccount, BackendLeafIndex, BackendMerklePath, BackendMerkleRoot, BackendNote,
            BackendNullifier, BackendTokenAmount, BackendTokenId, BackendTrapdoor, FrontendAccount,
            FrontendLeafIndex, FrontendMerklePath, FrontendMerkleRoot, FrontendNote,
            FrontendNullifier, FrontendTokenAmount, FrontendTokenId, FrontendTrapdoor,
        },
    };

    #[relation_object_definition]
    struct WithdrawRelation {
        #[constant]
        pub max_path_len: u8,

        // Public inputs
        #[public_input(frontend_type = "FrontendTokenAmount")]
        pub fee: BackendTokenAmount,
        #[public_input(frontend_type = "FrontendAccount", parse_with = "convert_account")]
        pub recipient: BackendAccount,
        #[public_input(frontend_type = "FrontendTokenId")]
        pub token_id: BackendTokenId,
        #[public_input(frontend_type = "FrontendNullifier")]
        pub old_nullifier: BackendNullifier,
        #[public_input(frontend_type = "FrontendNote", parse_with = "convert_hash")]
        pub new_note: BackendNote,
        #[public_input(frontend_type = "FrontendTokenAmount")]
        pub token_amount_out: BackendTokenAmount,
        #[public_input(frontend_type = "FrontendMerkleRoot", parse_with = "convert_hash")]
        pub merkle_root: BackendMerkleRoot,

        // Private inputs.
        #[private_input(frontend_type = "FrontendTrapdoor")]
        pub old_trapdoor: BackendTrapdoor,
        #[private_input(frontend_type = "FrontendTrapdoor")]
        pub new_trapdoor: BackendTrapdoor,
        #[private_input(frontend_type = "FrontendNullifier")]
        pub new_nullifier: BackendNullifier,
        #[private_input(frontend_type = "FrontendMerklePath", parse_with = "convert_vec")]
        pub merkle_path: BackendMerklePath,
        #[private_input(frontend_type = "FrontendLeafIndex")]
        pub leaf_index: BackendLeafIndex,
        #[private_input(frontend_type = "FrontendNote", parse_with = "convert_hash")]
        pub old_note: BackendNote,
        #[private_input(frontend_type = "FrontendTokenAmount")]
        pub whole_token_amount: BackendTokenAmount,
        #[private_input(frontend_type = "FrontendTokenAmount")]
        pub new_token_amount: BackendTokenAmount,
    }

    #[circuit_definition]
    fn generate_constraints() {
        //-----------------------------------------------
        // Baking `fee` and `recipient` into the circuit.
        //-----------------------------------------------
        let _fee = FpVar::new_input(ns!(cs, "fee"), || self.fee())?;
        let _recipient = FpVar::new_input(ns!(cs, "recipient"), || self.recipient())?;

        //------------------------------
        // Check the old note arguments.
        //------------------------------
        let old_note = FpVar::new_witness(ns!(cs, "old note"), || self.old_note())?;
        let token_id = FpVar::new_input(ns!(cs, "token id"), || self.token_id())?;
        let whole_token_amount =
            FpVar::new_witness(ns!(cs, "whole token amount"), || self.whole_token_amount())?;
        let old_trapdoor = FpVar::new_witness(ns!(cs, "old trapdoor"), || self.old_trapdoor())?;
        let old_nullifier = FpVar::new_input(ns!(cs, "old nullifier"), || self.old_nullifier())?;

        check_note(
            &token_id,
            &whole_token_amount,
            &old_trapdoor,
            &old_nullifier,
            &old_note,
        )?;

        //------------------------------
        // Check the new note arguments.
        //------------------------------
        let new_note = FpVar::new_input(ns!(cs, "new note"), || self.new_note())?;
        let new_token_amount =
            FpVar::new_witness(ns!(cs, "new token amount"), || self.new_token_amount())?;
        let new_trapdoor = FpVar::new_witness(ns!(cs, "new trapdoor"), || self.new_trapdoor())?;
        let new_nullifier = FpVar::new_witness(ns!(cs, "new nullifier"), || self.new_nullifier())?;

        check_note(
            &token_id,
            &new_token_amount,
            &new_trapdoor,
            &new_nullifier,
            &new_note,
        )?;

        //----------------------------------
        // Check the token values soundness.
        //----------------------------------
        let token_amount_out =
            FpVar::new_input(ns!(cs, "token amount out"), || self.token_amount_out())?;
        // some range checks for overflows?
        let token_sum = token_amount_out.add(new_token_amount);
        token_sum.enforce_equal(&whole_token_amount)?;

        //------------------------
        // Check the merkle proof.
        //------------------------
        let merkle_root = FpVar::new_input(ns!(cs, "merkle root"), || self.merkle_root())?;
        let path_shape = PathShapeVar::new_witness(ns!(cs, "path shape"), || {
            Ok((*self.max_path_len(), self.leaf_index().cloned()))
        })?;

        check_merkle_proof(
            merkle_root,
            path_shape,
            old_note,
            self.merkle_path().cloned().unwrap_or_default(),
            *self.max_path_len(),
            cs,
        )
    }
}

#[cfg(test)]
mod tests {
    use ark_bls12_381::Bls12_381;
    use ark_groth16::Groth16;
    use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem};
    use ark_snark::SNARK;

    use super::*;
    use crate::{
        shielder::note::{compute_note, compute_parent_hash},
        FrontendAccount,
    };

    const MAX_PATH_LEN: u8 = 4;

    fn get_circuit_with_full_input() -> WithdrawRelationWithFullInput {
        let token_id: FrontendTokenId = 1;

        let old_trapdoor: FrontendTrapdoor = 17;
        let old_nullifier: FrontendNullifier = 19;
        let whole_token_amount: FrontendTokenAmount = 10;

        let new_trapdoor: FrontendTrapdoor = 27;
        let new_nullifier: FrontendNullifier = 87;
        let new_token_amount: FrontendTokenAmount = 3;

        let token_amount_out: FrontendTokenAmount = 7;

        let old_note = compute_note(token_id, whole_token_amount, old_trapdoor, old_nullifier);
        let new_note = compute_note(token_id, new_token_amount, new_trapdoor, new_nullifier);

        //                                          merkle root
        //                placeholder                                        x
        //        1                          x                     x                         x
        //   2        3                x          x            x       x                 x       x
        // 4  *5*   6   7            x   x      x   x        x   x   x   x             x   x   x   x
        let leaf_index = 5;

        let zero_note = FrontendNote::default(); // x

        let sibling_note = compute_note(0, 1, 2, 3); // 4
        let parent_note = compute_parent_hash(sibling_note, old_note); // 2
        let uncle_note = compute_note(4, 5, 6, 7); // 3
        let grandpa_root = compute_parent_hash(parent_note, uncle_note); // 1

        let placeholder = compute_parent_hash(grandpa_root, zero_note);
        let merkle_root = compute_parent_hash(placeholder, zero_note);

        let merkle_path = vec![sibling_note, uncle_note];

        let fee: FrontendTokenAmount = 1;
        let recipient: FrontendAccount = [
            212, 53, 147, 199, 21, 253, 211, 28, 97, 20, 26, 189, 4, 169, 159, 214, 130, 44, 133,
            88, 133, 76, 205, 227, 154, 86, 132, 231, 165, 109, 162, 125,
        ];

        WithdrawRelationWithFullInput::new(
            MAX_PATH_LEN,
            fee,
            recipient,
            token_id,
            old_nullifier,
            new_note,
            token_amount_out,
            merkle_root,
            old_trapdoor,
            new_trapdoor,
            new_nullifier,
            merkle_path,
            leaf_index,
            old_note,
            whole_token_amount,
            new_token_amount,
        )
    }

    #[test]
    fn withdraw_constraints_correctness() {
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
    fn withdraw_proving_procedure() {
        let circuit_withouth_input = WithdrawRelationWithoutInput::new(MAX_PATH_LEN);

        let mut rng = ark_std::test_rng();
        let (pk, vk) =
            Groth16::<Bls12_381>::circuit_specific_setup(circuit_withouth_input, &mut rng).unwrap();

        let circuit = get_circuit_with_full_input();
        let proof = Groth16::prove(&pk, circuit, &mut rng).unwrap();

        let circuit: WithdrawRelationWithPublicInput = get_circuit_with_full_input().into();
        let input = circuit.serialize_public_input();
        let valid_proof = Groth16::verify(&vk, &input, &proof).unwrap();
        assert!(valid_proof);
    }

    #[test]
    fn neither_fee_nor_recipient_are_simplified_out() {
        let circuit_withouth_input = WithdrawRelationWithoutInput::new(MAX_PATH_LEN);

        let mut rng = ark_std::test_rng();
        let (pk, vk) =
            Groth16::<Bls12_381>::circuit_specific_setup(circuit_withouth_input, &mut rng).unwrap();

        let circuit = get_circuit_with_full_input();
        let proof = Groth16::prove(&pk, circuit, &mut rng).unwrap();

        let circuit: WithdrawRelationWithPublicInput = get_circuit_with_full_input().into();
        let true_input = circuit.serialize_public_input();
        let mut input_with_corrupted_fee = true_input.clone();
        input_with_corrupted_fee[0] = BackendTokenAmount::from(2);
        assert_ne!(true_input[0], input_with_corrupted_fee[0]);

        let valid_proof = Groth16::verify(&vk, &input_with_corrupted_fee, &proof).unwrap();
        assert!(!valid_proof);

        let mut input_with_corrupted_recipient = true_input.clone();
        let fake_recipient = [41; 32];
        input_with_corrupted_recipient[1] = convert_account(fake_recipient);
        assert_ne!(true_input[1], input_with_corrupted_recipient[1]);

        let valid_proof = Groth16::verify(&vk, &input_with_corrupted_recipient, &proof).unwrap();
        assert!(!valid_proof);
    }
}
