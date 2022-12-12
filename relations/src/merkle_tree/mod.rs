//! This module contains a relation for checking membership within a Merkle tree together with
//! all the auxiliary stuff (like special gadgets etc.).
//!
//! Currently, the SNARK here works with BLS12-381 curve: in particular, the circuit operates on
//! field `ark_bls12_381::Fr` - for brevity `Fr`. *HOWEVER*, hash functions use Twisted Edwards
//! curve (which is built atop the scalar field `Fr`). Therefore, keep in mind that
//! `ark_ed_on_bls12_381::Fq === ark_bls12_381::Fr`.
//! We will try to clean this up a bit soon.

mod gadgets;
mod hash_functions;
mod relation;
mod tree;

pub use relation::MerkleTreeRelation;
