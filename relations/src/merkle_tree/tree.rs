use ark_crypto_primitives::{
    crh::{pedersen::Parameters, TwoToOneCRH},
    merkle_tree::Config,
    MerkleTree, Path, CRH,
};
use ark_ed_on_bls12_381::EdwardsProjective;
use ark_std::rand::{prelude::StdRng, SeedableRng};

use crate::merkle_tree::hash_functions::{LeafHash, TwoToOneHash};

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct MerkleConfig;
impl Config for MerkleConfig {
    type LeafHash = LeafHash;
    type TwoToOneHash = TwoToOneHash;
}

/// A Merkle tree containing some bytes.
pub type SimpleMerkleTree = MerkleTree<MerkleConfig>;
/// The root of the byte Merkle tree.
pub type Root = <TwoToOneHash as TwoToOneCRH>::Output;
/// A membership proof for a given byte.
pub type SimplePath = Path<MerkleConfig>;

/// Creates a merkle tree from a vector of it's leaves
///
/// Returns a tuple of:
///  - the tree
///  - the parameters of leaf hashing function
///  - the parameters of node combining hashing function
pub fn new_tree(
    leaves: Vec<u8>,
    seed: [u8; 32],
) -> (
    SimpleMerkleTree,
    Parameters<EdwardsProjective>,
    Parameters<EdwardsProjective>,
) {
    let mut rng = StdRng::from_seed(seed);

    let leaf_crh_params = <LeafHash as CRH>::setup(&mut rng).unwrap();
    let two_to_one_crh_params = <TwoToOneHash as TwoToOneCRH>::setup(&mut rng).unwrap();

    let tree = SimpleMerkleTree::new(&leaf_crh_params, &two_to_one_crh_params, &leaves).unwrap();

    (tree, leaf_crh_params, two_to_one_crh_params)
}
