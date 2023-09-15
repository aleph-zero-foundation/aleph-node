use jf_primitives::merkle_tree::{
    prelude::RescueSparseMerkleTree, MerkleTreeScheme, UniversalMerkleTreeScheme,
};
use jf_relation::{Circuit, PlonkCircuit};
use num_bigint::BigUint;

use crate::{
    check_merkle_proof,
    note::{NoteGadget, NoteType, SourcedNote},
    shielder_types::{
        convert_array, LeafIndex, MerkleRoot, Note, Nullifier, TokenAmount, TokenId, Trapdoor,
    },
    CircuitField, MerkleProof, PlonkResult, PublicInput, Relation, MERKLE_TREE_HEIGHT,
};

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct MergeRelation {
    first_leaf_index: LeafIndex,
    first_merkle_path: MerkleProof,
    first_old_note: SourcedNote,
    merkle_root: MerkleRoot,
    new_note: SourcedNote,
    second_leaf_index: LeafIndex,
    second_merkle_path: MerkleProof,
    second_old_note: SourcedNote,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Default, Debug)]
pub struct MergePublicInput {
    pub first_old_nullifier: Nullifier,
    pub merkle_root: MerkleRoot,
    pub new_note: Note,
    pub second_old_nullifier: Nullifier,
    pub token_id: TokenId,
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct MergePrivateInput {
    pub first_leaf_index: LeafIndex,
    pub first_merkle_path: MerkleProof,
    pub first_old_note: Note,
    pub first_old_token_amount: TokenAmount,
    pub first_old_trapdoor: Trapdoor,
    pub new_nullifier: Nullifier,
    pub new_token_amount: TokenAmount,
    pub new_trapdoor: Trapdoor,
    pub second_leaf_index: LeafIndex,
    pub second_merkle_path: MerkleProof,
    pub second_old_note: Note,
    pub second_old_token_amount: TokenAmount,
    pub second_old_trapdoor: Trapdoor,
}

impl Default for MergePrivateInput {
    fn default() -> Self {
        let index = BigUint::from(0u64);
        let value = CircuitField::from(0u64);

        let merkle_tree =
            RescueSparseMerkleTree::from_kv_set(MERKLE_TREE_HEIGHT, [(index.clone(), value)])
                .unwrap();

        let (_, merkle_proof) = merkle_tree.lookup(&index).expect_ok().unwrap();

        Self {
            first_old_trapdoor: Default::default(),
            second_old_trapdoor: Default::default(),
            new_trapdoor: Default::default(),
            new_nullifier: Default::default(),
            first_merkle_path: merkle_proof.clone(),
            second_merkle_path: merkle_proof,
            first_leaf_index: Default::default(),
            second_leaf_index: Default::default(),
            first_old_note: Default::default(),
            second_old_note: Default::default(),
            first_old_token_amount: Default::default(),
            second_old_token_amount: Default::default(),
            new_token_amount: Default::default(),
        }
    }
}

impl MergeRelation {
    pub fn new(public: MergePublicInput, private: MergePrivateInput) -> Self {
        let first_old_note = SourcedNote {
            note: private.first_old_note,
            token_id: public.token_id,
            token_amount: private.first_old_token_amount,
            trapdoor: private.first_old_trapdoor,
            nullifier: public.first_old_nullifier,
            note_type: NoteType::Spend,
        };

        let second_old_note = SourcedNote {
            note: private.second_old_note,
            token_id: public.token_id,
            token_amount: private.second_old_token_amount,
            trapdoor: private.second_old_trapdoor,
            nullifier: public.second_old_nullifier,
            note_type: NoteType::Spend,
        };

        let new_note = SourcedNote {
            note: public.new_note,
            token_id: public.token_id,
            token_amount: private.new_token_amount,
            trapdoor: private.new_trapdoor,
            nullifier: private.new_nullifier,
            note_type: NoteType::Redeposit,
        };

        Self {
            first_old_note,
            second_old_note,
            new_note,
            merkle_root: public.merkle_root,
            first_merkle_path: private.first_merkle_path,
            first_leaf_index: private.first_leaf_index,
            second_merkle_path: private.second_merkle_path,
            second_leaf_index: private.second_leaf_index,
        }
    }
}

impl Default for MergeRelation {
    fn default() -> Self {
        Self::new(Default::default(), Default::default())
    }
}

impl PublicInput for MergeRelation {
    fn public_input(&self) -> Vec<CircuitField> {
        let mut public_input = Vec::new();

        public_input.extend(self.first_old_note.public_input());
        public_input.extend(self.second_old_note.public_input());
        public_input.extend(self.new_note.public_input());
        public_input.push(convert_array(self.merkle_root));

        public_input
    }
}

impl Relation for MergeRelation {
    fn generate_subcircuit(&self, circuit: &mut PlonkCircuit<CircuitField>) -> PlonkResult<()> {
        //------------------------------
        // first_old_note = H(token_id, first_old_token_amount, first_old_trapdoor, first_old_nullifier)
        //------------------------------
        let first_old_note_var = circuit.create_note_variable(&self.first_old_note)?;
        let first_old_note_token_amount_var = first_old_note_var.token_amount_var;
        circuit.enforce_note_preimage(first_old_note_var)?;

        //------------------------------
        // second_old_note = H(token_id, first_old_token_amount, first_old_trapdoor, first_old_nullifier)
        //------------------------------
        let second_old_note_var = circuit.create_note_variable(&self.second_old_note)?;
        let second_old_note_token_amount_var = second_old_note_var.token_amount_var;
        circuit.enforce_note_preimage(second_old_note_var)?;

        //------------------------------
        // new_note = H(token_id, new_token_amount, new_trapdoor, new_nullifier)
        //------------------------------
        let new_note_var = circuit.create_note_variable(&self.new_note)?;
        let new_note_token_amount_var = new_note_var.token_amount_var;
        circuit.enforce_note_preimage(new_note_var)?;

        //------------------------------
        //  first_merkle_path is a valid Merkle proof for first_old_note being present
        //  at first_leaf_index in a Merkle tree with merkle_root hash in the root
        //------------------------------
        check_merkle_proof(
            circuit,
            self.first_leaf_index,
            self.merkle_root,
            &self.first_merkle_path,
            true,
        )?;

        //------------------------------
        //  second_merkle_path is a valid Merkle proof for second_old_note being present
        //  at first_leaf_index in a Merkle tree with merkle_root hash in the root
        //------------------------------
        check_merkle_proof(
            circuit,
            self.second_leaf_index,
            self.merkle_root,
            &self.second_merkle_path,
            false,
        )?;

        //------------------------------
        //  new_token_amount = first_old_token_amount + second_old_token_amount
        //------------------------------
        let old_notes_token_amount_sum_var = circuit.add(
            first_old_note_token_amount_var,
            second_old_note_token_amount_var,
        )?;
        circuit.enforce_equal(old_notes_token_amount_sum_var, new_note_token_amount_var)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use ark_ff::PrimeField;
    use jf_primitives::merkle_tree::{
        prelude::RescueSparseMerkleTree, MerkleCommitment, MerkleTreeScheme,
        UniversalMerkleTreeScheme,
    };
    use jf_relation::Circuit;
    use num_bigint::BigUint;

    use super::*;
    use crate::shielder_types::compute_note;

    fn merge_relation() -> MergeRelation {
        let token_id = 1;

        let first_old_token_amount = 7;
        let first_old_trapdoor = [1; 4];
        let first_old_nullifier = [2; 4];

        let first_old_note = compute_note(
            token_id,
            first_old_token_amount,
            first_old_trapdoor,
            first_old_nullifier,
        );

        let second_old_token_amount = 3;
        let second_old_trapdoor = [3; 4];
        let second_old_nullifier = [4; 4];

        let second_old_note = compute_note(
            token_id,
            second_old_token_amount,
            second_old_trapdoor,
            second_old_nullifier,
        );

        let new_token_amount = first_old_token_amount + second_old_token_amount;
        let new_trapdoor = [5; 4];
        let new_nullifier = [6; 4];

        let new_note = compute_note(token_id, new_token_amount, new_trapdoor, new_nullifier);

        let first_leaf_index = 0u64;
        let first_uid = BigUint::from(first_leaf_index);
        let first_value = convert_array(first_old_note);

        let second_leaf_index = 1u64;
        let second_uid = BigUint::from(second_leaf_index);
        let second_value = convert_array(second_old_note);

        let tree = RescueSparseMerkleTree::from_kv_set(
            MERKLE_TREE_HEIGHT,
            &[
                (first_uid.clone(), first_value),
                (second_uid.clone(), second_value),
            ],
        )
        .expect("create Merkle tree from k-v pairs");

        let (first_value_retrieved, first_merkle_proof) = tree
            .lookup(&first_uid)
            .expect_ok()
            .expect("lookup first old note in Merkle tree");

        assert_eq!(first_value, first_value_retrieved);
        assert!(tree
            .verify(&first_uid, first_merkle_proof.clone())
            .expect("membership verified"));

        let (second_value_retrieved, second_merkle_proof) = tree
            .lookup(&second_uid)
            .expect_ok()
            .expect("lookup second old note in Merkle tree");

        assert_eq!(second_value, second_value_retrieved);
        assert!(tree
            .verify(&second_uid, second_merkle_proof.clone())
            .expect("membership verified"));

        let merkle_root = tree.commitment().digest().into_bigint().0;

        let public = MergePublicInput {
            token_id,
            first_old_nullifier,
            second_old_nullifier,
            new_note,
            merkle_root,
        };

        let private = MergePrivateInput {
            first_old_trapdoor,
            second_old_trapdoor,
            new_trapdoor,
            new_nullifier,
            first_merkle_path: first_merkle_proof,
            second_merkle_path: second_merkle_proof,
            first_leaf_index,
            second_leaf_index,
            first_old_note,
            second_old_note,
            first_old_token_amount,
            second_old_token_amount,
            new_token_amount,
        };

        MergeRelation::new(public, private)
    }

    fn merge_constraints_correctness(relation: MergeRelation) -> bool {
        let circuit = MergeRelation::generate_circuit(&relation).unwrap();

        match circuit.check_circuit_satisfiability(&relation.public_input()) {
            Ok(_) => true,
            Err(why) => {
                println!("circuit not satisfied: {}", why);
                false
            }
        }
    }

    #[test]
    fn merge_constraints_valid_circuit() {
        let relation = merge_relation();
        let constraints_correctness = merge_constraints_correctness(relation);
        assert!(constraints_correctness);
    }

    #[test]
    fn merge_constraints_invalid_first_old_note() {
        let mut invalid_relation = merge_relation();
        invalid_relation.first_old_note.token_amount += 1;

        let constraints_correctness = merge_constraints_correctness(invalid_relation);
        assert!(!constraints_correctness);
    }

    #[test]
    fn merge_constraints_invalid_second_old_note() {
        let mut invalid_relation = merge_relation();
        invalid_relation.second_old_note.token_amount += 1;

        let constraints_correctness = merge_constraints_correctness(invalid_relation);
        assert!(!constraints_correctness);
    }

    #[test]
    fn merge_constraints_invalid_new_note() {
        let mut invalid_relation = merge_relation();
        invalid_relation.new_note.token_amount += 1;

        let constraints_correctness = merge_constraints_correctness(invalid_relation);
        assert!(!constraints_correctness);
    }

    #[test]
    fn merge_constraints_invalid_first_leaf_index() {
        let mut invalid_relation = merge_relation();
        invalid_relation.first_leaf_index += 1;

        let constraints_correctness = merge_constraints_correctness(invalid_relation);
        assert!(!constraints_correctness);
    }

    #[test]
    fn merge_constraints_invalid_second_leaf_index() {
        let mut invalid_relation = merge_relation();
        invalid_relation.second_leaf_index += 1;

        let constraints_correctness = merge_constraints_correctness(invalid_relation);
        assert!(!constraints_correctness);
    }
}
