use core::ops::Add;

use ark_ff::BigInteger256;
use ark_r1cs_std::{alloc::AllocVar, eq::EqGadget, ToBytesGadget};
use ark_relations::{
    ns,
    r1cs::{
        ConstraintSynthesizer, ConstraintSystemRef, SynthesisError,
        SynthesisError::AssignmentMissing,
    },
};
use ark_std::{marker::PhantomData, vec, vec::Vec};

use super::{
    check_merkle_proof,
    note::check_note,
    types::{
        BackendLeafIndex, BackendMerklePath, BackendMerkleRoot, BackendNote, BackendNullifier,
        BackendTokenAmount, BackendTokenId, BackendTrapdoor, FrontendLeafIndex, FrontendMerklePath,
        FrontendMerkleRoot, FrontendNote, FrontendNullifier, FrontendTokenAmount, FrontendTokenId,
        FrontendTrapdoor,
    },
};
use crate::{
    environment::{CircuitField, FpVar},
    relation::{
        state::{FullInput, NoInput, OnlyPublicInput, State, WithPublicInput},
        GetPublicInput,
    },
};

/// 'DepositAndMerge' relation for the Shielder application.
///
/// It expresses the facts that:
///  - `old_note` is a prefix of the result of tangling together `token_id`, `old_token_amount`,
///    `old_trapdoor` and `old_nullifier`,
///  - `new_note` is a prefix of the result of tangling together `token_id`, `new_token_amount`,
///    `new_trapdoor` and `new_nullifier`,
///  - `new_token_amount = token_amount + old_token_amount`
///  - `merkle_path` is a valid Merkle proof for `old_note` being present at `leaf_index` in some
///    Merkle tree with `merkle_root` hash in the root
/// Additionally, the relation has one constant input, `max_path_len` which specifies upper bound
/// for the length of the merkle path (which is ~the height of the tree, ±1).
///
/// When providing a public input to proof verification, you should keep the order of variable
/// declarations in circuit, i.e.: `token_id`, `old_nullifier`, `new_note`, `token_amount`, `merkle_root`.

#[derive(Clone)]
pub struct DepositAndMergeRelation<S: State> {
    // Constant input.
    pub max_path_len: u8,

    // Public inputs
    pub token_id: Option<BackendTokenId>,
    pub token_amount: Option<BackendTokenAmount>,
    pub old_nullifier: Option<BackendNullifier>,
    pub merkle_root: Option<BackendMerkleRoot>,
    pub new_note: Option<BackendNote>,

    // Private inputs.
    pub old_trapdoor: Option<BackendTrapdoor>,
    pub new_trapdoor: Option<BackendTrapdoor>,
    pub new_nullifier: Option<BackendNullifier>,
    pub merkle_path: Option<BackendMerklePath>,
    pub leaf_index: Option<BackendLeafIndex>,
    pub old_note: Option<BackendNote>,
    pub old_token_amount: Option<BackendTokenAmount>,
    pub new_token_amount: Option<BackendTokenAmount>,

    _phantom: PhantomData<S>,
}

impl DepositAndMergeRelation<NoInput> {
    pub fn without_input(max_path_len: u8) -> Self {
        DepositAndMergeRelation {
            max_path_len,
            // Public inputs
            token_id: None,
            token_amount: None,
            old_nullifier: None,
            merkle_root: None,
            new_note: None,
            // Private inputs.
            old_trapdoor: None,
            new_trapdoor: None,
            new_nullifier: None,
            merkle_path: None,
            leaf_index: None,
            old_note: None,
            old_token_amount: None,
            new_token_amount: None,
            _phantom: PhantomData,
        }
    }
}

impl DepositAndMergeRelation<OnlyPublicInput> {
    #[allow(clippy::too_many_arguments)]
    pub fn with_public_input(
        max_path_len: u8,
        token_id: FrontendTokenId,
        token_amount: FrontendTokenAmount,
        old_nullifier: FrontendNullifier,
        merkle_root: FrontendMerkleRoot,
        new_note: FrontendNote,
    ) -> Self {
        DepositAndMergeRelation {
            max_path_len,
            // Public inputs
            token_id: Some(BackendTokenId::from(token_id)),
            token_amount: Some(BackendTokenAmount::from(token_amount)),
            old_nullifier: Some(BackendNullifier::from(old_nullifier)),
            merkle_root: Some(BackendMerkleRoot::from(BigInteger256::new(merkle_root))),
            new_note: Some(BackendNote::from(BigInteger256::new(new_note))),

            // Private inputs.
            old_trapdoor: None,
            new_trapdoor: None,
            new_nullifier: None,
            merkle_path: None,
            leaf_index: None,
            old_note: None,
            old_token_amount: None,
            new_token_amount: None,
            _phantom: PhantomData,
        }
    }
}

impl DepositAndMergeRelation<FullInput> {
    #[allow(clippy::too_many_arguments)]
    pub fn with_full_input(
        max_path_len: u8,
        token_id: FrontendTokenId,
        token_amount: FrontendTokenAmount,
        old_nullifier: FrontendNullifier,
        merkle_root: FrontendMerkleRoot,
        new_note: FrontendNote,
        old_trapdoor: FrontendTrapdoor,
        new_trapdoor: FrontendTrapdoor,
        new_nullifier: FrontendNullifier,
        merkle_path: FrontendMerklePath,
        leaf_index: FrontendLeafIndex,
        old_note: FrontendNote,
        old_token_amount: FrontendTokenAmount,
        new_token_amount: FrontendTokenAmount,
    ) -> Self {
        DepositAndMergeRelation {
            max_path_len,
            // Public inputs
            token_id: Some(BackendTokenId::from(token_id)),
            token_amount: Some(BackendTokenAmount::from(token_amount)),
            old_nullifier: Some(BackendNullifier::from(old_nullifier)),
            merkle_root: Some(BackendMerkleRoot::from(BigInteger256::new(merkle_root))),
            new_note: Some(BackendNote::from(BigInteger256::new(new_note))),
            // Private inputs.
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
            old_token_amount: Some(BackendTokenAmount::from(old_token_amount)),
            new_token_amount: Some(BackendTokenAmount::from(new_token_amount)),

            _phantom: PhantomData,
        }
    }
}

impl<S: State> ConstraintSynthesizer<CircuitField> for DepositAndMergeRelation<S> {
    fn generate_constraints(
        self,
        cs: ConstraintSystemRef<CircuitField>,
    ) -> Result<(), SynthesisError> {
        //------------------------------
        // Check the old note arguments.
        //------------------------------
        let token_id = FpVar::new_input(ns!(cs, "token id"), || {
            self.token_id.ok_or(AssignmentMissing)
        })?;
        let old_token_amount = FpVar::new_witness(ns!(cs, "old token amount"), || {
            self.old_token_amount.ok_or(AssignmentMissing)
        })?;
        let old_trapdoor = FpVar::new_witness(ns!(cs, "old trapdoor"), || {
            self.old_trapdoor.ok_or(AssignmentMissing)
        })?;
        let old_nullifier = FpVar::new_input(ns!(cs, "old nullifier"), || {
            self.old_nullifier.ok_or(AssignmentMissing)
        })?;
        let old_note = FpVar::new_witness(ns!(cs, "old note"), || {
            self.old_note.ok_or(AssignmentMissing)
        })?;

        check_note(
            &token_id,
            &old_token_amount,
            &old_trapdoor,
            &old_nullifier,
            &old_note,
        )?;

        //------------------------------
        // Check the new note arguments.
        //------------------------------
        let new_token_amount = FpVar::new_witness(ns!(cs, "new token amount"), || {
            self.new_token_amount.ok_or(AssignmentMissing)
        })?;
        let new_trapdoor = FpVar::new_witness(ns!(cs, "new trapdoor"), || {
            self.new_trapdoor.ok_or(AssignmentMissing)
        })?;
        let new_nullifier = FpVar::new_witness(ns!(cs, "new nullifier"), || {
            self.new_nullifier.ok_or(AssignmentMissing)
        })?;
        let new_note = FpVar::new_input(ns!(cs, "new note"), || {
            self.new_note.ok_or(AssignmentMissing)
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
        let token_amount = FpVar::new_input(ns!(cs, "token amount"), || {
            self.token_amount.ok_or(AssignmentMissing)
        })?;
        // some range checks for overflows?
        let token_sum = token_amount.add(old_token_amount);
        token_sum.enforce_equal(&new_token_amount)?;

        //------------------------
        // Check the merkle proof.
        //------------------------
        check_merkle_proof(
            self.merkle_root,
            self.leaf_index,
            old_note.to_bytes()?,
            self.merkle_path,
            self.max_path_len,
            cs,
        )
    }
}

impl<S: WithPublicInput> GetPublicInput<CircuitField> for DepositAndMergeRelation<S> {
    // The order here should match the order of registation inputs in generate_constraints
    fn public_input(&self) -> Vec<CircuitField> {
        vec![
            self.token_id.unwrap(),
            self.old_nullifier.unwrap(),
            self.new_note.unwrap(),
            self.token_amount.unwrap(),
            self.merkle_root.unwrap(),
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
    use crate::shielder::note::{compute_note, compute_parent_hash};

    const MAX_PATH_LEN: u8 = 10;

    fn get_circuit_with_full_input() -> DepositAndMergeRelation<FullInput> {
        let token_id: FrontendTokenId = 1;

        let old_trapdoor: FrontendTrapdoor = 17;
        let old_nullifier: FrontendNullifier = 19;
        let old_token_amount: FrontendTokenAmount = 7;

        let new_trapdoor: FrontendTrapdoor = 27;
        let new_nullifier: FrontendNullifier = 87;
        let new_token_amount: FrontendTokenAmount = 10;

        let token_amount: FrontendTokenAmount = 3;

        let old_note = compute_note(token_id, old_token_amount, old_trapdoor, old_nullifier);
        let new_note = compute_note(token_id, new_token_amount, new_trapdoor, new_nullifier);

        // Our leaf has a left bro. Their parent has a right bro. Our grandpa is the root.
        let leaf_index = 5;

        let sibling_note = compute_note(0, 1, 2, 3);
        let parent_note = compute_parent_hash(sibling_note, old_note);
        let uncle_note = compute_note(4, 5, 6, 7);
        let merkle_root = compute_parent_hash(parent_note, uncle_note);

        let merkle_path = vec![sibling_note, uncle_note];

        DepositAndMergeRelation::with_full_input(
            MAX_PATH_LEN,
            token_id,
            token_amount,
            old_nullifier,
            merkle_root,
            new_note,
            old_trapdoor,
            new_trapdoor,
            new_nullifier,
            merkle_path,
            leaf_index,
            old_note,
            old_token_amount,
            new_token_amount,
        )
    }

    #[test]
    fn deposit_and_merge_constraints_correctness() {
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
    fn deposit_and_merge_proving_procedure() {
        let circuit_wo_input = DepositAndMergeRelation::without_input(MAX_PATH_LEN);

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
