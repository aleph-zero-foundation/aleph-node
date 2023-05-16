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
    #[cfg(feature = "circuit")]
    use {
        crate::shielder::{
            check_merkle_proof, note_var::NoteVarBuilder, path_shape_var::PathShapeVar,
            token_amount_var::TokenAmountVar,
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
        convert_account, convert_hash, convert_vec,
        types::{
            BackendAccount, BackendLeafIndex, BackendMerklePath, BackendMerkleRoot, BackendNote,
            BackendNullifier, BackendTokenAmount, BackendTokenId, BackendTrapdoor, FrontendAccount,
            FrontendLeafIndex, FrontendMerklePath, FrontendMerkleRoot, FrontendNote,
            FrontendNullifier, FrontendTokenAmount, FrontendTokenId, FrontendTrapdoor,
        },
    };

    #[relation_object_definition]
    #[derive(Clone, Debug)]
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
        #[public_input(frontend_type = "FrontendNullifier", parse_with = "convert_hash")]
        pub old_nullifier: BackendNullifier,
        #[public_input(frontend_type = "FrontendNote", parse_with = "convert_hash")]
        pub new_note: BackendNote,
        #[public_input(frontend_type = "FrontendTokenAmount")]
        pub token_amount_out: BackendTokenAmount,
        #[public_input(frontend_type = "FrontendMerkleRoot", parse_with = "convert_hash")]
        pub merkle_root: BackendMerkleRoot,

        // Private inputs.
        #[private_input(frontend_type = "FrontendTrapdoor", parse_with = "convert_hash")]
        pub old_trapdoor: BackendTrapdoor,
        #[private_input(frontend_type = "FrontendTrapdoor", parse_with = "convert_hash")]
        pub new_trapdoor: BackendTrapdoor,
        #[private_input(frontend_type = "FrontendNullifier", parse_with = "convert_hash")]
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

    #[cfg(feature = "circuit")]
    #[circuit_definition]
    fn generate_constraints() {
        //-----------------------------------------------
        // Baking `fee` and `recipient` into the circuit.
        //-----------------------------------------------
        let _fee = TokenAmountVar::new_input(ns!(cs, "fee"), || self.fee())?;
        let _recipient = FpVar::new_input(ns!(cs, "recipient"), || self.recipient())?;

        //------------------------------
        // Check the old note arguments.
        //------------------------------
        let old_note = NoteVarBuilder::new(cs.clone())
            .with_note(self.old_note(), Witness)?
            .with_token_id(self.token_id(), Input)?
            .with_token_amount(self.whole_token_amount(), Witness)?
            .with_trapdoor(self.old_trapdoor(), Witness)?
            .with_nullifier(self.old_nullifier(), Input)?
            .build()?;

        //------------------------------
        // Check the new note arguments.
        //------------------------------
        let new_note = NoteVarBuilder::new(cs.clone())
            .with_token_id_var(old_note.token_id.clone())
            .with_note(self.new_note(), Input)?
            .with_token_amount(self.new_token_amount(), Witness)?
            .with_trapdoor(self.new_trapdoor(), Witness)?
            .with_nullifier(self.new_nullifier(), Witness)?
            .build()?;

        //----------------------------------
        // Check the token values soundness.
        //----------------------------------
        let token_amount_out =
            TokenAmountVar::new_input(ns!(cs, "token amount out"), || self.token_amount_out())?;
        let token_sum = token_amount_out.add(new_note.token_amount)?;
        token_sum.enforce_equal(&old_note.token_amount)?;

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
            old_note.note,
            self.merkle_path().cloned().unwrap_or_default(),
            *self.max_path_len(),
            cs,
        )
    }
}

#[cfg(all(test, feature = "circuit"))]
mod tests {
    use std::ops::Neg;

    use ark_bls12_381::Bls12_381;
    use ark_ff::One;
    use ark_groth16::Groth16;
    use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem};
    use ark_snark::SNARK;

    use super::*;
    use crate::shielder::{
        note::{compute_note, compute_parent_hash},
        types::{BackendNote, BackendTokenAmount, FrontendAccount},
    };

    const MAX_PATH_LEN: u8 = 4;

    fn get_circuit_with_full_input() -> WithdrawRelationWithFullInput {
        let token_id: FrontendTokenId = 1;

        let old_trapdoor: FrontendTrapdoor = [17; 4];
        let old_nullifier: FrontendNullifier = [19; 4];
        let whole_token_amount: FrontendTokenAmount = 10;

        let new_trapdoor: FrontendTrapdoor = [27; 4];
        let new_nullifier: FrontendNullifier = [87; 4];
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

        let sibling_note = compute_note(0, 1, [2; 4], [3; 4]); // 4
        let parent_note = compute_parent_hash(sibling_note, old_note); // 2
        let uncle_note = compute_note(4, 5, [6; 4], [7; 4]); // 3
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
        let circuit_without_input = WithdrawRelationWithoutInput::new(MAX_PATH_LEN);

        let mut rng = ark_std::test_rng();
        let (pk, vk) =
            Groth16::<Bls12_381>::circuit_specific_setup(circuit_without_input, &mut rng).unwrap();

        let circuit = get_circuit_with_full_input();
        let proof = Groth16::prove(&pk, circuit, &mut rng).unwrap();

        let circuit = WithdrawRelationWithPublicInput::from(get_circuit_with_full_input());
        let input = circuit.serialize_public_input();
        let valid_proof = Groth16::verify(&vk, &input, &proof).unwrap();
        assert!(valid_proof);
    }

    #[test]
    fn neither_fee_nor_recipient_are_simplified_out() {
        let circuit_without_input = WithdrawRelationWithoutInput::new(MAX_PATH_LEN);

        let mut rng = ark_std::test_rng();
        let (pk, vk) =
            Groth16::<Bls12_381>::circuit_specific_setup(circuit_without_input, &mut rng).unwrap();

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

    #[test]
    fn cannot_create_sneaky_note() {
        let circuit_without_input = WithdrawRelationWithoutInput::new(MAX_PATH_LEN);

        let mut rng = ark_std::test_rng();
        let (pk, vk) =
            Groth16::<Bls12_381>::circuit_specific_setup(circuit_without_input, &mut rng).unwrap();

        let mut circuit = get_circuit_with_full_input();
        // We want to take one token more than deposited...
        circuit.token_amount_out = circuit.whole_token_amount + BackendTokenAmount::one();
        // ... hence we need to leave in Shielder -1 token ...
        circuit.new_token_amount = BackendTokenAmount::one().neg();
        // ... and compute new sneaky note.
        circuit.new_note = BackendNote::new(ark_ff::BigInteger256([
            875544533870975309,
            17340113879898921273,
            17290319916917063854,
            4489249721891001805,
        ]));

        let proof = Groth16::prove(&pk, circuit.clone(), &mut rng).unwrap();

        let circuit = WithdrawRelationWithPublicInput::from(circuit);
        let input = circuit.serialize_public_input();
        let valid_proof = Groth16::verify(&vk, &input, &proof).unwrap();
        // Without `enforce_cmp` in `TokenAmountVar` this proof is valid!
        assert!(!valid_proof);
    }
}
