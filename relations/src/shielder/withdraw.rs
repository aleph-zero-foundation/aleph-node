use core::ops::{Add, Div};

use ark_ff::{BigInteger, BigInteger256, Zero};
use ark_r1cs_std::{alloc::AllocVar, eq::EqGadget, fields::FieldVar, R1CSVar, ToBytesGadget};
use ark_relations::{
    ns,
    r1cs::{
        ConstraintSynthesizer, ConstraintSystemRef, SynthesisError,
        SynthesisError::{AssignmentMissing, UnconstrainedVariable},
    },
};
use ark_std::{marker::PhantomData, vec, vec::Vec};

use super::{
    note::check_note,
    tangle::tangle_in_field,
    types::{
        BackendAccount, BackendLeafIndex, BackendMerklePath, BackendMerkleRoot, BackendNote,
        BackendNullifier, BackendTokenAmount, BackendTokenId, BackendTrapdoor, ByteVar,
        FrontendAccount, FrontendLeafIndex, FrontendMerklePath, FrontendMerkleRoot, FrontendNote,
        FrontendNullifier, FrontendTokenAmount, FrontendTokenId, FrontendTrapdoor,
    },
};
use crate::{
    environment::{CircuitField, FpVar},
    relation::{
        state::{FullInput, NoInput, OnlyPublicInput, State, WithPublicInput},
        GetPublicInput,
    },
};

/// 'Withdraw' relation for the Shielder application.
///
/// It expresses the facts that:
///  - `new_note` is a prefix of the result of tangling together `token_id`, `whole_token_amount`,
///    `old_trapdoor` and `old_nullifier`,
///  - `old_note` is a prefix of the result of tangling together `token_id`, `new_token_amount`,
///    `new_trapdoor` and `new_nullifier`,
///  - `new_token_amount + token_amount_out = whole_token_amount`
///  - `merkle_path` is a valid Merkle proof for `old_note` being present at `leaf_index` in some
///    Merkle tree with `merkle_root` hash in the root
/// It also includes two artificial inputs `fee` and `recipient` just to strengthen the application
/// security by treating them as public inputs (and thus integral part of the SNARK).
/// Additionally, the relation has one constant input, `max_path_len` which specifies upper bound
/// for the length of the merkle path (which is ~the height of the tree, ±1).
///
/// When providing a public input to proof verification, you should keep the order of variable
/// declarations in circuit, i.e.: `fee`, `recipient`, `token_id`, `old_nullifier`, `new_note`,
/// `token_amount_out`, `merkle_root`.
#[derive(Clone)]
pub struct WithdrawRelation<S: State> {
    // Constant input.
    pub max_path_len: u8,

    // Public inputs.
    pub fee: Option<BackendTokenAmount>,
    pub recipient: Option<BackendAccount>,
    pub token_id: Option<BackendTokenId>,
    pub old_nullifier: Option<BackendNullifier>,
    pub new_note: Option<BackendNote>,
    pub token_amount_out: Option<BackendTokenAmount>,
    pub merkle_root: Option<BackendMerkleRoot>,

    // Private inputs.
    pub old_trapdoor: Option<BackendTrapdoor>,
    pub new_trapdoor: Option<BackendTrapdoor>,
    pub new_nullifier: Option<BackendNullifier>,
    pub merkle_path: Option<BackendMerklePath>,
    pub leaf_index: Option<BackendLeafIndex>,
    pub old_note: Option<BackendNote>,
    pub whole_token_amount: Option<BackendTokenAmount>,
    pub new_token_amount: Option<BackendTokenAmount>,

    _phantom: PhantomData<S>,
}

impl WithdrawRelation<NoInput> {
    pub fn without_input(max_path_len: u8) -> Self {
        WithdrawRelation {
            max_path_len,
            fee: None,
            recipient: None,
            token_id: None,
            old_nullifier: None,
            new_note: None,
            token_amount_out: None,
            merkle_root: None,
            old_trapdoor: None,
            new_trapdoor: None,
            new_nullifier: None,
            merkle_path: None,
            leaf_index: None,
            old_note: None,
            whole_token_amount: None,
            new_token_amount: None,
            _phantom: PhantomData,
        }
    }
}

impl WithdrawRelation<OnlyPublicInput> {
    #[allow(clippy::too_many_arguments)]
    pub fn with_public_input(
        max_path_len: u8,
        fee: FrontendTokenAmount,
        recipient: FrontendAccount,
        token_id: FrontendTokenId,
        old_nullifier: FrontendNullifier,
        new_note: FrontendNote,
        token_amount_out: FrontendTokenAmount,
        merkle_root: FrontendMerkleRoot,
    ) -> Self {
        // todo: move frontend-backend conversion to common place (even without strong types)
        WithdrawRelation {
            max_path_len,
            fee: Some(BackendTokenAmount::from(fee)),
            recipient: Some(BackendAccount::new(BigInteger256::new([
                u64::from_le_bytes(recipient[0..8].try_into().unwrap()),
                u64::from_le_bytes(recipient[8..16].try_into().unwrap()),
                u64::from_le_bytes(recipient[16..24].try_into().unwrap()),
                u64::from_le_bytes(recipient[24..32].try_into().unwrap()),
            ]))),
            token_id: Some(BackendTokenId::from(token_id)),
            old_nullifier: Some(BackendNullifier::from(old_nullifier)),
            new_note: Some(BackendNote::from(BigInteger256::new(new_note))),
            token_amount_out: Some(BackendTokenAmount::from(token_amount_out)),
            merkle_root: Some(BackendMerkleRoot::from(BigInteger256::new(merkle_root))),

            old_trapdoor: None,
            new_trapdoor: None,
            new_nullifier: None,
            merkle_path: None,
            leaf_index: None,
            old_note: None,
            whole_token_amount: None,
            new_token_amount: None,
            _phantom: PhantomData,
        }
    }
}

impl WithdrawRelation<FullInput> {
    #[allow(clippy::too_many_arguments)]
    pub fn with_full_input(
        max_path_len: u8,
        fee: FrontendTokenAmount,
        recipient: FrontendAccount,
        token_id: FrontendTokenId,
        old_nullifier: FrontendNullifier,
        new_note: FrontendNote,
        token_amount_out: FrontendTokenAmount,
        merkle_root: FrontendMerkleRoot,
        old_trapdoor: FrontendTrapdoor,
        new_trapdoor: FrontendTrapdoor,
        new_nullifier: FrontendNullifier,
        merkle_path: FrontendMerklePath,
        leaf_index: FrontendLeafIndex,
        old_note: FrontendNote,
        whole_token_amount: FrontendTokenAmount,
        new_token_amount: FrontendTokenAmount,
    ) -> Self {
        WithdrawRelation {
            max_path_len,
            fee: Some(BackendTokenAmount::from(fee)),
            recipient: Some(BackendAccount::new(BigInteger256::new([
                u64::from_le_bytes(recipient[0..8].try_into().unwrap()),
                u64::from_le_bytes(recipient[8..16].try_into().unwrap()),
                u64::from_le_bytes(recipient[16..24].try_into().unwrap()),
                u64::from_le_bytes(recipient[24..32].try_into().unwrap()),
            ]))),
            token_id: Some(BackendTokenId::from(token_id)),
            old_nullifier: Some(BackendNullifier::from(old_nullifier)),
            new_note: Some(BackendNote::from(BigInteger256::new(new_note))),
            token_amount_out: Some(BackendTokenAmount::from(token_amount_out)),
            merkle_root: Some(BackendMerkleRoot::from(BigInteger256::new(merkle_root))),

            old_trapdoor: Some(BackendTrapdoor::from(old_trapdoor)),
            new_trapdoor: Some(BackendTrapdoor::from(new_trapdoor)),
            new_nullifier: Some(BackendNullifier::from(new_nullifier)),
            merkle_path: Some(
                merkle_path
                    .iter()
                    .map(|node| BackendNote::from(BigInteger256::new(*node)))
                    .collect(),
            ),
            leaf_index: Some(BackendLeafIndex::from(leaf_index)),
            old_note: Some(BackendNote::from(BigInteger256::new(old_note))),
            whole_token_amount: Some(BackendTokenAmount::from(whole_token_amount)),
            new_token_amount: Some(BackendTokenAmount::from(new_token_amount)),

            _phantom: PhantomData,
        }
    }
}

impl<S: State> ConstraintSynthesizer<CircuitField> for WithdrawRelation<S> {
    fn generate_constraints(
        self,
        cs: ConstraintSystemRef<CircuitField>,
    ) -> Result<(), SynthesisError> {
        //-----------------------------------------------
        // Baking `fee` and `recipient` into the circuit.
        //-----------------------------------------------
        let _fee = FpVar::new_input(ns!(cs, "fee"), || self.fee.ok_or(AssignmentMissing))?;
        let _recipient = FpVar::new_input(ns!(cs, "recipient"), || {
            self.recipient.ok_or(AssignmentMissing)
        })?;

        //------------------------------
        // Check the old note arguments.
        //------------------------------
        let old_note = FpVar::new_witness(ns!(cs, "old note"), || {
            self.old_note.ok_or(AssignmentMissing)
        })?;
        let token_id = FpVar::new_input(ns!(cs, "token id"), || {
            self.token_id.ok_or(AssignmentMissing)
        })?;
        let whole_token_amount = FpVar::new_witness(ns!(cs, "whole token amount"), || {
            self.whole_token_amount.ok_or(AssignmentMissing)
        })?;
        let old_trapdoor = FpVar::new_witness(ns!(cs, "old trapdoor"), || {
            self.old_trapdoor.ok_or(AssignmentMissing)
        })?;
        let old_nullifier = FpVar::new_input(ns!(cs, "old nullifier"), || {
            self.old_nullifier.ok_or(AssignmentMissing)
        })?;

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
        let new_note = FpVar::new_input(ns!(cs, "new note"), || {
            self.new_note.ok_or(AssignmentMissing)
        })?;
        let new_token_amount = FpVar::new_witness(ns!(cs, "new token amount"), || {
            self.new_token_amount.ok_or(AssignmentMissing)
        })?;
        let new_trapdoor = FpVar::new_witness(ns!(cs, "new trapdoor"), || {
            self.new_trapdoor.ok_or(AssignmentMissing)
        })?;
        let new_nullifier = FpVar::new_witness(ns!(cs, "new nullifier"), || {
            self.new_nullifier.ok_or(AssignmentMissing)
        })?;

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
        let token_amount_out = FpVar::new_input(ns!(cs, "token amount out"), || {
            self.token_amount_out.ok_or(AssignmentMissing)
        })?;
        // some range checks for overflows?
        let token_sum = token_amount_out.add(new_token_amount);
        token_sum.enforce_equal(&whole_token_amount)?;

        //------------------------
        // Check the merkle proof.
        //------------------------
        let merkle_root = FpVar::new_input(ns!(cs, "merkle root"), || {
            self.merkle_root.ok_or(AssignmentMissing)
        })?;
        let mut leaf_index = FpVar::new_witness(ns!(cs, "leaf index"), || {
            self.leaf_index.ok_or(AssignmentMissing)
        })?;

        let mut current_hash_bytes = old_note.to_bytes()?;
        let mut hash_bytes = vec![current_hash_bytes.clone()];
        let path = self.merkle_path.unwrap_or_default();

        if path.len() > self.max_path_len as usize {
            return Err(UnconstrainedVariable);
        }

        let zero = CircuitField::zero();

        for i in 0..self.max_path_len {
            let sibling = FpVar::new_witness(ns!(cs, "merkle path node"), || {
                Ok(path.get(i as usize).unwrap_or(&zero))
            })?;
            let bytes: Vec<ByteVar> = if leaf_index.value().unwrap_or_default().0.is_even() {
                [current_hash_bytes.clone(), sibling.to_bytes()?].concat()
            } else {
                [sibling.to_bytes()?, current_hash_bytes.clone()].concat()
            };

            current_hash_bytes = tangle_in_field::<2>(bytes)?;
            hash_bytes.push(current_hash_bytes.clone());

            leaf_index = FpVar::constant(
                leaf_index
                    .value()
                    .unwrap_or_default()
                    .div(CircuitField::from(2)),
            );
        }

        for (a, b) in merkle_root
            .to_bytes()?
            .iter()
            .zip(hash_bytes[path.len()].iter())
        {
            a.enforce_equal(b)?;
        }

        Ok(())
    }
}

impl<S: WithPublicInput> GetPublicInput<CircuitField> for WithdrawRelation<S> {
    fn public_input(&self) -> Vec<CircuitField> {
        [
            self.fee.unwrap(),
            self.recipient.unwrap(),
            self.token_id.unwrap(),
            self.old_nullifier.unwrap(),
            self.new_note.unwrap(),
            self.token_amount_out.unwrap(),
            self.merkle_root.unwrap(),
        ]
        .to_vec()
    }
}

#[cfg(test)]
mod tests {
    use ark_bls12_381::Bls12_381;
    use ark_groth16::Groth16;
    use ark_relations::r1cs::ConstraintSystem;
    use ark_snark::SNARK;

    use super::*;
    use crate::shielder::note::{compute_note, compute_parent_hash};

    const MAX_PATH_LEN: u8 = 10;

    fn get_circuit_with_full_input() -> WithdrawRelation<FullInput> {
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

        // Our leaf has a left bro. Their parent has a right bro. Our grandpa is the root.
        let leaf_index = 5;

        let sibling_note = compute_note(0, 1, 2, 3);
        let parent_note = compute_parent_hash(sibling_note, old_note);
        let uncle_note = compute_note(4, 5, 6, 7);
        let merkle_root = compute_parent_hash(parent_note, uncle_note);

        let merkle_path = vec![sibling_note, uncle_note];

        let fee: FrontendTokenAmount = 1;
        let recipient: FrontendAccount = [
            212, 53, 147, 199, 21, 253, 211, 28, 97, 20, 26, 189, 4, 169, 159, 214, 130, 44, 133,
            88, 133, 76, 205, 227, 154, 86, 132, 231, 165, 109, 162, 125,
        ];

        WithdrawRelation::with_full_input(
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
        let circuit_wo_input = WithdrawRelation::without_input(MAX_PATH_LEN);

        let mut rng = ark_std::test_rng();
        let (pk, vk) =
            Groth16::<Bls12_381>::circuit_specific_setup(circuit_wo_input, &mut rng).unwrap();

        let circuit = get_circuit_with_full_input();
        let input = circuit.public_input();

        let proof = Groth16::prove(&pk, circuit, &mut rng).unwrap();
        let valid_proof = Groth16::verify(&vk, &input, &proof).unwrap();
        assert!(valid_proof);
    }

    #[test]
    fn neither_fee_nor_recipient_are_simplified_out() {
        let circuit_wo_input = WithdrawRelation::without_input(MAX_PATH_LEN);

        let mut rng = ark_std::test_rng();
        let (pk, vk) =
            Groth16::<Bls12_381>::circuit_specific_setup(circuit_wo_input, &mut rng).unwrap();

        let circuit = get_circuit_with_full_input();
        let true_input = circuit.public_input();
        let proof = Groth16::prove(&pk, circuit, &mut rng).unwrap();

        let mut input_with_corrupted_fee = true_input.clone();
        input_with_corrupted_fee[0] = BackendTokenAmount::from(2);
        assert_ne!(true_input[0], input_with_corrupted_fee[0]);

        let valid_proof = Groth16::verify(&vk, &input_with_corrupted_fee, &proof).unwrap();
        assert!(!valid_proof);

        let mut input_with_corrupted_recipient = true_input.clone();
        let fake_recipient = [41; 32];
        // todo: implement casting between backend and frontend types
        input_with_corrupted_recipient[1] = BackendAccount::new(BigInteger256::new([
            u64::from_le_bytes(fake_recipient[0..8].try_into().unwrap()),
            u64::from_le_bytes(fake_recipient[8..16].try_into().unwrap()),
            u64::from_le_bytes(fake_recipient[16..24].try_into().unwrap()),
            u64::from_le_bytes(fake_recipient[24..32].try_into().unwrap()),
        ]));
        assert_ne!(true_input[1], input_with_corrupted_recipient[1]);

        let valid_proof = Groth16::verify(&vk, &input_with_corrupted_recipient, &proof).unwrap();
        assert!(!valid_proof);
    }
}
