use ark_crypto_primitives::{
    crh::{TwoToOneCRH, TwoToOneCRHGadget},
    PathVar, CRH,
};
use ark_r1cs_std::{boolean::Boolean, eq::EqGadget, prelude::AllocVar, uint8::UInt8};
use ark_relations::r1cs::{
    ConstraintSynthesizer, ConstraintSystemRef, SynthesisError, SynthesisError::AssignmentMissing,
};
use ark_std::{marker::PhantomData, string::String, vec, vec::Vec};

use crate::{
    byte_to_bits,
    environment::CircuitField,
    merkle_tree::{
        gadgets::{LeafHashGadget, LeafHashParamsVar, TwoToOneHashGadget, TwoToOneHashParamsVar},
        hash_functions::{LeafHash, TwoToOneHash},
        tree::{new_tree, tree_parameters, MerkleConfig, Root, SimplePath},
    },
    relation::{
        state::{FullInput, NoInput, OnlyPublicInput, State, WithPublicInput},
        GetPublicInput,
    },
    string_to_padded_bytes,
};

/// The R1CS equivalent of the the Merkle tree root.
pub type RootVar = <TwoToOneHashGadget as TwoToOneCRHGadget<TwoToOneHash, CircuitField>>::OutputVar;

/// The R1CS equivalent of the the Merkle tree path.
pub type SimplePathVar = PathVar<MerkleConfig, LeafHashGadget, TwoToOneHashGadget, CircuitField>;

/// Relation for checking membership in a Merkle tree.
///
/// `MerkleTreeRelation` represents a membership proof for a single leaf in a tree on n leaves.
#[derive(Clone)]
pub struct MerkleTreeRelation<S: State> {
    /// Private witness.
    pub merkle_path: Option<SimplePath>,

    /// Root of the tree (public input).
    pub root: Option<Root>,
    /// Leaf which membership is to be proven (public input).
    pub leaf: Option<u8>,

    /// Collision-resistant hash function for leafs (constant parameter).
    pub leaf_crh_params: <LeafHash as CRH>::Parameters,
    /// Collision-resistant hash function translating child hashes to parent hash
    /// (constant parameter).
    pub two_to_one_crh_params: <TwoToOneHash as TwoToOneCRH>::Parameters,

    _phantom: PhantomData<S>,
}

impl MerkleTreeRelation<NoInput> {
    pub fn without_input(seed: Option<String>) -> Self {
        let (leaf_crh_params, two_to_one_crh_params) =
            tree_parameters(string_to_padded_bytes(seed.unwrap_or_default()));

        MerkleTreeRelation {
            merkle_path: None,
            root: None,
            leaf: None,
            leaf_crh_params,
            two_to_one_crh_params,
            _phantom: PhantomData,
        }
    }
}

impl MerkleTreeRelation<OnlyPublicInput> {
    pub fn with_public_input(root: Root, leaf: u8, seed: Option<String>) -> Self {
        let (leaf_crh_params, two_to_one_crh_params) =
            tree_parameters(string_to_padded_bytes(seed.unwrap_or_default()));

        MerkleTreeRelation {
            merkle_path: None,
            root: Some(root),
            leaf: Some(leaf),
            leaf_crh_params,
            two_to_one_crh_params,
            _phantom: PhantomData,
        }
    }
}

impl MerkleTreeRelation<FullInput> {
    pub fn with_full_input(leaves: Vec<u8>, leaf: u8, seed: Option<String>) -> Self {
        let leaf_idx = leaves
            .iter()
            .position(|&element| element == leaf)
            .expect("Leaf is not in the tree leaves");

        let (tree, leaf_crh_params, two_to_one_crh_params) =
            new_tree(leaves, string_to_padded_bytes(seed.unwrap_or_default()));

        MerkleTreeRelation {
            merkle_path: Some(tree.generate_proof(leaf_idx).unwrap()),
            root: Some(tree.root()),
            leaf: Some(leaf),
            leaf_crh_params,
            two_to_one_crh_params,
            _phantom: PhantomData,
        }
    }
}

impl<S: State> ConstraintSynthesizer<CircuitField> for MerkleTreeRelation<S> {
    fn generate_constraints(
        self,
        cs: ConstraintSystemRef<CircuitField>,
    ) -> Result<(), SynthesisError> {
        let path = SimplePathVar::new_witness(ark_relations::ns!(cs, "path"), || {
            self.merkle_path.ok_or_else(|| {
            #[cfg(feature = "std")]
            if cs.is_in_setup_mode() {
                eprintln!("Unfortunately, `MerkleTreeRelation` requires path even for keys generation. Blame `arkworks`.");
            }
            AssignmentMissing
        })
        })?;

        let root = RootVar::new_input(ark_relations::ns!(cs, "root_var"), || {
            self.root.ok_or(AssignmentMissing)
        })?;
        let leaf = UInt8::new_input(ark_relations::ns!(cs, "leaf_var"), || {
            self.leaf.ok_or(AssignmentMissing)
        })?;

        let leaf_crh_params = LeafHashParamsVar::new_constant(cs.clone(), &self.leaf_crh_params)?;
        let two_to_one_crh_params =
            TwoToOneHashParamsVar::new_constant(cs, &self.two_to_one_crh_params)?;

        let leaf_bytes = vec![leaf; 1];

        let is_member = path.verify_membership(
            &leaf_crh_params,
            &two_to_one_crh_params,
            &root,
            &leaf_bytes.as_slice(),
        )?;
        is_member.enforce_equal(&Boolean::TRUE)?;

        Ok(())
    }
}

impl<S: WithPublicInput> GetPublicInput<CircuitField> for MerkleTreeRelation<S> {
    fn public_input(&self) -> Vec<CircuitField> {
        [
            vec![self.root.unwrap()],
            byte_to_bits(&self.leaf.unwrap()).to_vec(),
        ]
        .concat()
    }
}
