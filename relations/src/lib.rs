#![cfg_attr(not(feature = "std"), no_std)]

mod environment;
mod linear;
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
pub use linear::{
    LinearEquationRelationWithFullInput, LinearEquationRelationWithPublicInput,
    LinearEquationRelationWithoutInput,
};
pub use preimage::{
    preimage_proving, PreimageRelationWithFullInput, PreimageRelationWithPublicInput,
    PreimageRelationWithoutInput,
};
pub use relation::GetPublicInput;
pub use serialization::serialize;
pub use shielder::{
    bytes_from_note, compute_note, compute_parent_hash, note_from_bytes, note_var::NoteVarBuilder,
    types::*, DepositAndMergeRelationWithFullInput, DepositAndMergeRelationWithPublicInput,
    DepositAndMergeRelationWithoutInput, DepositRelationWithFullInput,
    DepositRelationWithPublicInput, DepositRelationWithoutInput, MergeRelationWithFullInput,
    MergeRelationWithPublicInput, MergeRelationWithoutInput, WithdrawRelationWithFullInput,
    WithdrawRelationWithPublicInput, WithdrawRelationWithoutInput,
};
pub use utils::*;
pub use xor::{XorRelationWithFullInput, XorRelationWithPublicInput, XorRelationWithoutInput};
