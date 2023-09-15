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
pub struct DepositAndMergeRelation {
    leaf_index: LeafIndex,
    merkle_path: MerkleProof,
    merkle_root: MerkleRoot,
    new_note: SourcedNote,
    old_note: SourcedNote,
    deposit_token_amount: TokenAmount,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Default, Debug)]
pub struct DepositAndMergePublicInput {
    pub deposit_token_amount: TokenAmount,
    pub merkle_root: MerkleRoot,
    pub new_note: Note,
    pub old_nullifier: Nullifier,
    pub token_id: TokenId,
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct DepositAndMergePrivateInput {
    pub leaf_index: LeafIndex,
    pub merkle_path: MerkleProof,
    pub new_nullifier: Nullifier,
    pub new_token_amount: TokenAmount,
    pub new_trapdoor: Trapdoor,
    pub old_note: Note,
    pub old_token_amount: TokenAmount,
    pub old_trapdoor: Trapdoor,
}

impl Default for DepositAndMergePrivateInput {
    fn default() -> Self {
        let index = BigUint::from(0u64);
        let value = CircuitField::from(0u64);

        let merkle_tree =
            RescueSparseMerkleTree::from_kv_set(MERKLE_TREE_HEIGHT, [(index.clone(), value)])
                .unwrap();

        let (_, merkle_proof) = merkle_tree.lookup(&index).expect_ok().unwrap();

        Self {
            old_trapdoor: Default::default(),
            new_trapdoor: Default::default(),
            new_nullifier: Default::default(),
            merkle_path: merkle_proof,
            leaf_index: Default::default(),
            old_note: Default::default(),
            old_token_amount: Default::default(),
            new_token_amount: Default::default(),
        }
    }
}

impl DepositAndMergeRelation {
    pub fn new(public: DepositAndMergePublicInput, private: DepositAndMergePrivateInput) -> Self {
        let old_note = SourcedNote {
            note: private.old_note,
            token_id: public.token_id,
            token_amount: private.old_token_amount,
            trapdoor: private.old_trapdoor,
            nullifier: public.old_nullifier,
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
            old_note,
            new_note,
            merkle_path: private.merkle_path,
            leaf_index: private.leaf_index,
            merkle_root: public.merkle_root,
            deposit_token_amount: public.deposit_token_amount,
        }
    }
}

impl Default for DepositAndMergeRelation {
    fn default() -> Self {
        Self::new(Default::default(), Default::default())
    }
}

impl PublicInput for DepositAndMergeRelation {
    fn public_input(&self) -> Vec<CircuitField> {
        let mut public_input = Vec::new();

        public_input.extend(self.old_note.public_input());
        public_input.extend(self.new_note.public_input());
        public_input.push(convert_array(self.merkle_root));

        public_input
    }
}

impl Relation for DepositAndMergeRelation {
    fn generate_subcircuit(&self, circuit: &mut PlonkCircuit<CircuitField>) -> PlonkResult<()> {
        //------------------------------
        // old_note = H(token_id, old_token_amount, old_trapdoor, old_nullifier)
        //------------------------------

        let old_note_var = circuit.create_note_variable(&self.old_note)?;
        let old_note_token_amount_var = old_note_var.token_amount_var;
        circuit.enforce_note_preimage(old_note_var)?;

        //------------------------------
        // new_note = H(token_id, new_token_amount, new_trapdoor, new_nullifier)
        //------------------------------

        let new_note_var = circuit.create_note_variable(&self.new_note)?;
        let new_note_token_amount_var = new_note_var.token_amount_var;
        circuit.enforce_note_preimage(new_note_var)?;

        //------------------------------
        //  merkle_path is a valid Merkle proof for old_note being present
        //  at leaf_index in a Merkle tree with merkle_root hash in the root
        //------------------------------

        check_merkle_proof(
            circuit,
            self.leaf_index,
            self.merkle_root,
            &self.merkle_path,
            true,
        )?;

        //------------------------------
        //  new_token_amount = deposit_token_amount + old_token_amount
        //------------------------------

        let deposit_token_amount_var = circuit.create_variable(self.deposit_token_amount.into())?;
        let token_sum_var = circuit.add(old_note_token_amount_var, deposit_token_amount_var)?;
        circuit.enforce_equal(token_sum_var, new_note_token_amount_var)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use ark_ff::PrimeField;
    use jf_plonk::{
        proof_system::{PlonkKzgSnark, UniversalSNARK},
        transcript::StandardTranscript,
    };
    use jf_primitives::merkle_tree::{
        prelude::RescueSparseMerkleTree, MerkleCommitment, MerkleTreeScheme,
        UniversalMerkleTreeScheme,
    };
    use jf_relation::Circuit;
    use num_bigint::BigUint;

    use super::*;
    use crate::{generate_srs, shielder_types::compute_note, Curve};

    fn deposit_and_merge_relation() -> DepositAndMergeRelation {
        let token_id = 1;
        let deposit_token_amount = 3;

        let old_token_amount = 7;
        let old_trapdoor = [1; 4];
        let old_nullifier = [2; 4];

        let old_note = compute_note(token_id, old_token_amount, old_trapdoor, old_nullifier);

        let new_token_amount = deposit_token_amount + old_token_amount;
        let new_trapdoor = [3; 4];
        let new_nullifier = [4; 4];

        let new_note = compute_note(token_id, new_token_amount, new_trapdoor, new_nullifier);

        let leaf_index = 0u64;
        let uid = BigUint::from(leaf_index);
        let value = convert_array(old_note);

        let tree = RescueSparseMerkleTree::from_kv_set(MERKLE_TREE_HEIGHT, &[(uid.clone(), value)])
            .expect("create Merkle tree from k-v pairs");

        let (value_retrieved, merkle_proof) = tree
            .lookup(&uid)
            .expect_ok()
            .expect("lookup old note in Merkle tree");

        assert_eq!(value, value_retrieved);
        assert!(tree
            .verify(&uid, merkle_proof.clone())
            .expect("membership verified"));

        let merkle_root = tree.commitment().digest().into_bigint().0;

        let public = DepositAndMergePublicInput {
            deposit_token_amount,
            merkle_root,
            new_note,
            old_nullifier,
            token_id,
        };

        let private = DepositAndMergePrivateInput {
            leaf_index,
            merkle_path: merkle_proof,
            new_nullifier,
            new_token_amount,
            new_trapdoor,
            old_note,
            old_token_amount,
            old_trapdoor,
        };

        DepositAndMergeRelation::new(public, private)
    }

    fn is_correct(relation: DepositAndMergeRelation) -> bool {
        let circuit = DepositAndMergeRelation::generate_circuit(&relation).unwrap();

        match circuit.check_circuit_satisfiability(&relation.public_input()) {
            Ok(_) => true,
            Err(why) => {
                println!("circuit not satisfied: {}", why);
                false
            }
        }
    }

    #[test]
    fn test_valid_relation() {
        let valid_relation = deposit_and_merge_relation();
        assert!(is_correct(valid_relation));
    }

    #[test]
    fn test_invalid_relation() {
        let mut invalid_relation = deposit_and_merge_relation();
        invalid_relation.leaf_index = 1;
        assert!(!is_correct(invalid_relation));
    }

    #[test]
    fn proving_procedure() {
        let rng = &mut jf_utils::test_rng();
        let srs = generate_srs(10_000, rng).unwrap();

        let (pk, vk) = DepositAndMergeRelation::generate_keys(&srs).unwrap();

        let relation = deposit_and_merge_relation();
        let proof = relation.generate_proof(&pk, rng).unwrap();

        let public_input = relation.public_input();

        assert!(PlonkKzgSnark::<Curve>::verify::<StandardTranscript>(
            &vk,
            &public_input,
            &proof,
            None
        )
        .is_ok());
    }
}
