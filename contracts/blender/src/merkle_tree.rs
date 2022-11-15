use ink_env::{Clear, Hash};
use ink_prelude::vec::Vec;
use ink_storage::{
    traits::{SpreadAllocate, SpreadLayout},
    Mapping,
};
#[cfg(feature = "std")]
use scale_info::TypeInfo;

/// Temporary implementation of two-to-one hashing function.
fn kinder_blender(left: &Hash, right: &Hash) -> Hash {
    left.as_ref()
        .iter()
        .cloned()
        .zip(right.as_ref().iter().cloned())
        .map(|(l, r)| l ^ r)
        .collect::<Vec<_>>()
        .as_slice()
        .try_into()
        .unwrap()
}

/// Simplified binary tree that represents a Merkle tree over some set of hashes.
///
/// It has `LEAVES` leaves (thus `2 * LEAVES - 1` nodes in general). `LEAVES` must be power of `2`.
#[derive(SpreadAllocate, SpreadLayout)]
#[cfg_attr(feature = "std", derive(TypeInfo, ink_storage::traits::StorageLayout))]
pub struct MerkleTree<const LEAVES: u32> {
    /// Node values (root is at [1], children are in [2n] and [2n+1]).
    nodes: Mapping<u32, Hash>,
    /// Marker of the first 'non-occupied' leaf.
    next_free_leaf: u32,
}

impl<const LEAVES: u32> Default for MerkleTree<LEAVES> {
    fn default() -> Self {
        if !LEAVES.is_power_of_two() {
            panic!("Please have 2^n leaves")
        }

        Self {
            nodes: Mapping::default(),
            next_free_leaf: LEAVES,
        }
    }
}

impl<const LEAVES: u32> MerkleTree<LEAVES> {
    /// Get the value from the root node.
    ///
    /// Returns `None` if the tree is empty (`Self::add` has not been called yet).
    pub fn root(&self) -> Hash {
        self.nodes.get(1).unwrap_or_else(Hash::clear)
    }

    /// Add `value` to the first 'non-occupied' leaf.
    ///
    /// Returns `Err(())` iff there are no free leafs. Otherwise, returns the leaf index.
    pub fn add(&mut self, value: Hash) -> Result<u32, ()> {
        if self.next_free_leaf == 2 * LEAVES {
            return Err(());
        }

        self.nodes.insert(self.next_free_leaf, &value);

        let mut parent = self.next_free_leaf / 2;
        while parent > 0 {
            let left_child = &self.nodes.get(2 * parent).unwrap_or_else(Hash::clear);
            let right_child = &self.nodes.get(2 * parent + 1).unwrap_or_else(Hash::clear);
            self.nodes
                .insert(parent, &kinder_blender(left_child, right_child));
            parent /= 2;
        }

        self.next_free_leaf += 1;

        Ok(self.next_free_leaf - 1)
    }
}
