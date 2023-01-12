#![cfg_attr(not(feature = "std"), no_std)]

mod environment;
mod linear;
mod merkle_tree;
mod preimage;
mod relation;
mod serialization;
mod shielder;
mod utils;
mod xor;

pub use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, Result, SynthesisError};
pub use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
pub use environment::{
    CircuitField, Groth16, Marlin, MarlinPolynomialCommitment, NonUniversalSystem, ProvingSystem,
    RawKeys, UniversalSystem, GM17,
};
pub use linear::LinearEquationRelation;
pub use merkle_tree::{MerkleTreeRelation, Root};
pub use preimage::PreimageRelation;
pub use relation::GetPublicInput;
pub use serialization::serialize;
pub use shielder::{
    bytes_from_note, compute_note, compute_parent_hash, note_from_bytes, types::*, DepositRelation,
    WithdrawRelation,
};
pub use utils::*;
pub use xor::XorRelation;
