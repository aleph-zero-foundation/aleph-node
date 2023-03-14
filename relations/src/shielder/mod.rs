//! This module contains relations that are the core of the Shielder application: `deposit`, `deposit_and_merge` and
//! `withdraw`. It also exposes some functions and types that might be useful for input generation.

mod deposit;
mod deposit_and_merge;
mod merge;
mod note;
pub mod note_var;
mod path_shape_var;
pub mod types;
mod withdraw;

use ark_ff::{BigInteger256, PrimeField, Zero};
use ark_r1cs_std::{alloc::AllocVar, eq::EqGadget};
use ark_relations::{
    ns,
    r1cs::{ConstraintSystemRef, SynthesisError, SynthesisError::UnconstrainedVariable},
};
use ark_std::vec::Vec;
pub use deposit::{
    DepositRelationWithFullInput, DepositRelationWithPublicInput, DepositRelationWithoutInput,
};
pub use deposit_and_merge::{
    DepositAndMergeRelationWithFullInput, DepositAndMergeRelationWithPublicInput,
    DepositAndMergeRelationWithoutInput,
};
pub use merge::{
    MergeRelationWithFullInput, MergeRelationWithPublicInput, MergeRelationWithoutInput,
};
pub use note::{bytes_from_note, compute_note, compute_parent_hash, note_from_bytes};
use types::BackendMerklePath;
pub use types::{
    FrontendMerklePath as MerklePath, FrontendMerkleRoot as MerkleRoot, FrontendNote as Note,
    FrontendNullifier as Nullifier, FrontendTokenAmount as TokenAmount, FrontendTokenId as TokenId,
    FrontendTrapdoor as Trapdoor,
};
pub use withdraw::{
    WithdrawRelationWithFullInput, WithdrawRelationWithPublicInput, WithdrawRelationWithoutInput,
};

use crate::{
    environment::{CircuitField, FpVar},
    shielder::path_shape_var::PathShapeVar,
};

pub fn convert_hash(front: [u64; 4]) -> CircuitField {
    CircuitField::new(BigInteger256::new(front))
}

fn convert_vec(front: Vec<[u64; 4]>) -> Vec<CircuitField> {
    front.into_iter().map(convert_hash).collect()
}

fn convert_account(front: [u8; 32]) -> CircuitField {
    CircuitField::from_le_bytes_mod_order(&front)
}

fn check_merkle_proof(
    merkle_root: FpVar,
    path_shape: PathShapeVar,
    leaf: FpVar,
    path: BackendMerklePath,
    max_path_len: u8,
    cs: ConstraintSystemRef<CircuitField>,
) -> Result<(), SynthesisError> {
    if path.len() > max_path_len as usize {
        return Err(UnconstrainedVariable);
    }
    if path_shape.len() != max_path_len as usize {
        return Err(UnconstrainedVariable);
    }

    let mut current_note = leaf;
    let zero_note = CircuitField::zero();

    for i in 0..max_path_len as usize {
        let sibling = FpVar::new_witness(ns!(cs, "merkle path node"), || {
            Ok(path.get(i).unwrap_or(&zero_note))
        })?;

        let left = path_shape[i].select(&current_note, &sibling)?;
        let right = path_shape[i].select(&sibling, &current_note)?;

        current_note = liminal_ark_poseidon::circuit::two_to_one_hash(
            cs.clone(),
            [left.clone(), right.clone()],
        )?;
    }

    merkle_root.enforce_equal(&current_note)
}
