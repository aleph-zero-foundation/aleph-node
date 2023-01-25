//! This module contains two relations that are the core of the Shielder application: `deposit` and
//! `withdraw`. It also exposes some functions and types that might be useful for input generation.
//!
//! Currently, instead of using some real hash function, we chose to incorporate a simple tangling
//! algorithm. Essentially, it is a procedure that just mangles a byte sequence.

mod deposit;
mod deposit_and_merge;
mod note;
mod tangle;
pub mod types;
mod withdraw;

use core::ops::Div;

use ark_ff::{BigInteger, Zero};
use ark_r1cs_std::{
    alloc::AllocVar, eq::EqGadget, fields::FieldVar, uint8::UInt8, R1CSVar, ToBytesGadget,
};
use ark_relations::{
    ns,
    r1cs::{
        ConstraintSystemRef, SynthesisError,
        SynthesisError::{AssignmentMissing, UnconstrainedVariable},
    },
};
use ark_std::{vec, vec::Vec};
pub use deposit::{
    DepositRelationWithFullInput, DepositRelationWithPublicInput, DepositRelationWithoutInput,
};
pub use deposit_and_merge::DepositAndMergeRelation;
pub use note::{bytes_from_note, compute_note, compute_parent_hash, note_from_bytes};
use types::{BackendLeafIndex, BackendMerklePath, BackendMerkleRoot, ByteVar};
pub use types::{
    FrontendMerklePath as MerklePath, FrontendMerkleRoot as MerkleRoot, FrontendNote as Note,
    FrontendNullifier as Nullifier, FrontendTokenAmount as TokenAmount, FrontendTokenId as TokenId,
    FrontendTrapdoor as Trapdoor,
};
pub use withdraw::WithdrawRelation;

use crate::environment::{CircuitField, FpVar};

fn check_merkle_proof(
    merkle_root: Option<BackendMerkleRoot>,
    leaf_index: Option<BackendLeafIndex>,
    leaf_bytes: Vec<UInt8<CircuitField>>,
    merkle_path: Option<BackendMerklePath>,
    max_path_len: u8,
    cs: ConstraintSystemRef<CircuitField>,
) -> Result<(), SynthesisError> {
    let path = merkle_path.unwrap_or_default();
    if path.len() > max_path_len as usize {
        return Err(UnconstrainedVariable);
    }

    let zero = CircuitField::zero();

    let merkle_root = FpVar::new_input(ns!(cs, "merkle root"), || {
        merkle_root.ok_or(AssignmentMissing)
    })?;
    let mut leaf_index = FpVar::new_witness(ns!(cs, "leaf index"), || {
        leaf_index.ok_or(AssignmentMissing)
    })?;

    let mut current_hash_bytes = leaf_bytes;
    let mut hash_bytes = vec![current_hash_bytes.clone()];

    for i in 0..max_path_len {
        let sibling = FpVar::new_witness(ns!(cs, "merkle path node"), || {
            Ok(path.get(i as usize).unwrap_or(&zero))
        })?;
        let bytes: Vec<ByteVar> = if leaf_index.value().unwrap_or_default().0.is_even() {
            [current_hash_bytes.clone(), sibling.to_bytes()?].concat()
        } else {
            [sibling.to_bytes()?, current_hash_bytes.clone()].concat()
        };

        current_hash_bytes = tangle::tangle_in_field::<2>(bytes)?;
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
