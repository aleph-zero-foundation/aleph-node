use jf_primitives::merkle_tree::{
    prelude::RescueSparseMerkleTree, MerkleTreeScheme, UniversalMerkleTreeScheme,
};
use jf_relation::{Circuit, PlonkCircuit};
use num_bigint::BigUint;

use crate::{
    check_merkle_proof,
    note::{NoteGadget, NoteType, SourcedNote},
    shielder_types::{
        convert_account, convert_array, Account, LeafIndex, MerkleRoot, Note, Nullifier,
        TokenAmount, TokenId, Trapdoor,
    },
    CircuitField, MerkleProof, PlonkResult, PublicInput, Relation, MERKLE_TREE_HEIGHT,
};

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct WithdrawRelation {
    spend_note: SourcedNote,
    redeposit_note: SourcedNote,
    fee: TokenAmount,
    recipient: Account,
    token_amount_out: TokenAmount,
    merkle_root: MerkleRoot,
    merkle_proof: MerkleProof,
    leaf_index: LeafIndex,
}

impl Default for WithdrawRelation {
    fn default() -> Self {
        Self::new(Default::default(), Default::default())
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Default, Debug)]
pub struct WithdrawPublicInput {
    pub fee: TokenAmount,
    pub recipient: Account,
    pub token_id: TokenId,
    pub spend_nullifier: Nullifier,
    pub token_amount_out: TokenAmount,
    pub merkle_root: MerkleRoot,
    pub deposit_note: Note,
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct WithdrawPrivateInput {
    pub spend_trapdoor: Trapdoor,
    pub deposit_trapdoor: Trapdoor,
    pub deposit_nullifier: Nullifier,
    pub merkle_proof: MerkleProof,
    pub leaf_index: LeafIndex,
    pub spend_note: Note,
    pub whole_token_amount: TokenAmount,
    pub deposit_token_amount: TokenAmount,
}

impl Default for WithdrawPrivateInput {
    fn default() -> Self {
        let uid = BigUint::from(0u64);
        let elem = CircuitField::from(0u64);
        let mt =
            RescueSparseMerkleTree::from_kv_set(MERKLE_TREE_HEIGHT, [(uid.clone(), elem)]).unwrap();
        let (_, merkle_proof) = mt.lookup(&uid).expect_ok().unwrap();

        Self {
            spend_trapdoor: Default::default(),
            deposit_trapdoor: Default::default(),
            deposit_nullifier: Default::default(),
            merkle_proof,
            leaf_index: Default::default(),
            spend_note: Default::default(),
            whole_token_amount: Default::default(),
            deposit_token_amount: Default::default(),
        }
    }
}

impl WithdrawRelation {
    pub fn new(public: WithdrawPublicInput, private: WithdrawPrivateInput) -> Self {
        let spend_note = SourcedNote {
            note: private.spend_note,
            token_id: public.token_id,
            token_amount: private.whole_token_amount,
            trapdoor: private.spend_trapdoor,
            nullifier: public.spend_nullifier,
            note_type: NoteType::Spend,
        };
        let redeposit_note = SourcedNote {
            note: public.deposit_note,
            token_id: public.token_id,
            token_amount: private.deposit_token_amount,
            trapdoor: private.deposit_trapdoor,
            nullifier: private.deposit_nullifier,
            note_type: NoteType::Redeposit,
        };

        Self {
            spend_note,
            redeposit_note,
            fee: public.fee,
            recipient: public.recipient,
            token_amount_out: public.token_amount_out,
            merkle_root: public.merkle_root,
            merkle_proof: private.merkle_proof,
            leaf_index: private.leaf_index,
        }
    }
}

impl PublicInput for WithdrawRelation {
    fn public_input(&self) -> Vec<CircuitField> {
        let mut public_input = vec![
            self.fee.into(),
            convert_account(self.recipient),
            self.token_amount_out.into(),
        ];
        public_input.extend(self.spend_note.public_input());
        public_input.extend(self.redeposit_note.public_input());
        public_input.push(convert_array(self.merkle_root));

        public_input
    }
}

impl Relation for WithdrawRelation {
    fn generate_subcircuit(&self, circuit: &mut PlonkCircuit<CircuitField>) -> PlonkResult<()> {
        let fee_var = circuit.create_public_variable(self.fee.into())?;
        circuit.enforce_leq_constant(fee_var, CircuitField::from(u128::MAX))?;
        let _recipient_var = circuit.create_public_variable(convert_account(self.recipient))?;
        let token_amount_out_var = circuit.create_public_variable(self.token_amount_out.into())?;
        circuit.enforce_leq_constant(token_amount_out_var, CircuitField::from(u128::MAX))?;

        let spend_note_var = circuit.create_note_variable(&self.spend_note)?;
        let whole_token_amount_var = spend_note_var.token_amount_var;
        let spend_token_id_var = spend_note_var.token_id_var;
        circuit.enforce_note_preimage(spend_note_var)?;

        let deposit_note_var = circuit.create_note_variable(&self.redeposit_note)?;
        let deposit_amount_var = deposit_note_var.token_amount_var;
        let deposit_token_id_var = deposit_note_var.token_id_var;
        circuit.enforce_note_preimage(deposit_note_var)?;

        circuit.enforce_equal(deposit_token_id_var, spend_token_id_var)?;

        let token_sum_var = circuit.add(token_amount_out_var, deposit_amount_var)?;
        circuit.enforce_equal(token_sum_var, whole_token_amount_var)?;

        check_merkle_proof(
            circuit,
            self.leaf_index,
            self.merkle_root,
            &self.merkle_proof,
            true,
        )?;

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

    use crate::{
        generate_srs,
        shielder_types::{compute_note, convert_account, convert_array},
        withdraw::{WithdrawPrivateInput, WithdrawPublicInput, WithdrawRelation},
        CircuitField, Curve, PublicInput, Relation, MERKLE_TREE_HEIGHT,
    };

    fn relation() -> WithdrawRelation {
        let token_id = 1;
        let whole_token_amount = 10;
        let spend_trapdoor = [1; 4];
        let spend_nullifier = [2; 4];
        let spend_note = compute_note(
            token_id,
            whole_token_amount,
            spend_trapdoor,
            spend_nullifier,
        );

        let deposit_token_amount = 7;
        let deposit_trapdoor = [3; 4];
        let deposit_nullifier = [4; 4];
        let deposit_note = compute_note(
            token_id,
            deposit_token_amount,
            deposit_trapdoor,
            deposit_nullifier,
        );

        let leaf_index = 0u64;
        let uid = BigUint::from(leaf_index);
        let elem = convert_array(spend_note);
        let mt = RescueSparseMerkleTree::from_kv_set(MERKLE_TREE_HEIGHT, &[(uid.clone(), elem)])
            .unwrap();
        let (retrieved_elem, merkle_proof) = mt.lookup(&uid).expect_ok().unwrap();
        assert_eq!(retrieved_elem, elem);
        assert!(mt.verify(&uid, merkle_proof.clone()).expect("succeed"));
        let merkle_root = mt.commitment().digest().into_bigint().0;

        let public_input = WithdrawPublicInput {
            fee: 1,
            recipient: [7; 32],
            token_id,
            spend_nullifier,
            token_amount_out: 3,
            merkle_root,
            deposit_note,
        };

        let private_input = WithdrawPrivateInput {
            spend_trapdoor,
            deposit_trapdoor,
            deposit_nullifier,
            merkle_proof,
            leaf_index,
            spend_note,
            whole_token_amount,
            deposit_token_amount,
        };

        WithdrawRelation::new(public_input, private_input)
    }

    #[test]
    fn withdraw_constraints_correctness() {
        let relation = relation();
        let circuit = WithdrawRelation::generate_circuit(&relation).unwrap();
        circuit
            .check_circuit_satisfiability(&relation.public_input())
            .unwrap();
    }

    #[test]
    fn withdraw_constraints_incorrectness_with_wrong_note() {
        let mut relation = relation();
        relation.spend_note.note[0] += 1;
        let circuit = WithdrawRelation::generate_circuit(&relation).unwrap();
        assert!(circuit
            .check_circuit_satisfiability(&relation.public_input())
            .is_err());
    }

    #[test]
    fn withdraw_proving_procedure() {
        let rng = &mut jf_utils::test_rng();
        let srs = generate_srs(17_000, rng).unwrap();

        let (pk, vk) = WithdrawRelation::generate_keys(&srs).unwrap();

        let relation = relation();
        let proof = relation.generate_proof(&pk, rng).unwrap();

        let public_input = relation.public_input();

        PlonkKzgSnark::<Curve>::verify::<StandardTranscript>(&vk, &public_input, &proof, None)
            .unwrap();
    }

    #[test]
    fn neither_fee_nor_recipient_are_simplified_out() {
        let rng = &mut jf_utils::test_rng();
        let srs = generate_srs(17_000, rng).unwrap();

        let (pk, vk) = WithdrawRelation::generate_keys(&srs).unwrap();

        let relation = relation();
        let true_input = relation.public_input();
        let proof = relation.generate_proof(&pk, rng).unwrap();

        let mut input_with_corrupted_fee = true_input.clone();
        input_with_corrupted_fee[0] = CircuitField::from(2u64);
        assert_ne!(true_input[0], input_with_corrupted_fee[0]);

        let invalid_proof = PlonkKzgSnark::<Curve>::verify::<StandardTranscript>(
            &vk,
            &input_with_corrupted_fee,
            &proof,
            None,
        );
        assert!(invalid_proof.is_err());

        let mut input_with_corrupted_recipient = true_input.clone();
        let fake_recipient = [41; 32];
        input_with_corrupted_recipient[1] = convert_account(fake_recipient);
        assert_ne!(true_input[1], input_with_corrupted_recipient[1]);

        let invalid_proof = PlonkKzgSnark::<Curve>::verify::<StandardTranscript>(
            &vk,
            &input_with_corrupted_recipient,
            &proof,
            None,
        );
        assert!(invalid_proof.is_err());
    }
}
