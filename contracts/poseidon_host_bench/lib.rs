#![cfg_attr(not(feature = "std"), no_std)]

#[ink::contract(env = baby_liminal_extension::BabyLiminalEnvironment)]
mod poseidon_host_bench {
    use ink::storage::Mapping;
    use openbrush::traits::Storage;
    type MerkleHash = [u64; 4];

    #[ink(storage)]
    #[derive(Default, Storage)]
    pub struct PoseidonHostBench {
        hashes: Mapping<u32, MerkleHash>,
        next_free_leaf: u32,
    }

    impl PoseidonHostBench {
        #[ink(constructor)]
        pub fn new(max_leaves: u32) -> Self {
            if !max_leaves.is_power_of_two() {
                panic!("Please have 2^n leaves")
            }

            let mut this = PoseidonHostBench {
                next_free_leaf: max_leaves,
                ..Default::default()
            };

            this.create_new_leaf([1, 7, 2, 9]);

            this
        }

        #[ink(message)]
        pub fn hash(&mut self) {
            self.two_to_one_hash([2, 1, 3, 7], [1, 7, 2, 9]);
        }

        #[ink(message)]
        pub fn generate_path(&mut self) {
            self.create_new_leaf([2, 1, 3, 7])
        }

        fn tree_value(&self, idx: u32) -> MerkleHash {
            self.hashes.get(idx).unwrap_or_default()
        }

        fn create_new_leaf(&mut self, value: MerkleHash) {
            self.hashes.insert(self.next_free_leaf, &value);

            let mut parent = self.next_free_leaf / 2;
            while parent > 0 {
                let left_child = self.tree_value(2 * parent);
                let right_child = self.tree_value(2 * parent + 1);
                let parent_hash = self.two_to_one_hash(left_child, right_child);
                self.hashes.insert(parent, &parent_hash);
                parent /= 2;
            }

            self.next_free_leaf += 1;
        }

        fn two_to_one_hash(&self, x: MerkleHash, y: MerkleHash) -> MerkleHash {
            self.env().extension().poseidon_two_to_one([x, y])
        }
    }
}
