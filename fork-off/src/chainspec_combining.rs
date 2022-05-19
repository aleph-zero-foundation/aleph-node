use std::{collections::HashMap, ops::Index};

use log::info;

use crate::{Storage, StoragePath};

type StorageKeyHash = String;

fn hash_storage_prefix(storage_path: &StoragePath) -> StorageKeyHash {
    let modules = storage_path.0.split('.');
    let hashes = modules.flat_map(|module| sp_io::hashing::twox_128(module.as_bytes()));
    format!("0x{}", hex::encode(hashes.collect::<Vec<_>>()))
}

fn is_prefix_of(shorter: &str, longer: &str) -> bool {
    longer.starts_with(shorter)
}

#[derive(Default)]
struct PathCounter {
    map: HashMap<StoragePath, usize>,
}

impl PathCounter {
    pub fn bump(&mut self, path: &StoragePath) {
        *self.map.entry(path.clone()).or_default() += 1;
    }
}

impl Index<&StoragePath> for PathCounter {
    type Output = usize;

    fn index(&self, path: &StoragePath) -> &Self::Output {
        &self.map[path]
    }
}

pub fn combine_states(
    mut state: Storage,
    initial_state: Storage,
    storage_to_keep: Vec<StoragePath>,
) -> Storage {
    let storage_prefixes = storage_to_keep
        .iter()
        .map(|path| (path.clone(), hash_storage_prefix(path)))
        .collect::<Vec<_>>();

    let mut removed_per_path_count = PathCounter::default();
    let mut added_per_path_cnt = PathCounter::default();

    state.retain(|k, _v| {
        match storage_prefixes
            .iter()
            .find(|(_, prefix)| is_prefix_of(prefix, k))
        {
            Some((path, _)) => {
                removed_per_path_count.bump(path);
                false
            }
            None => true,
        }
    });

    for (k, v) in initial_state.iter() {
        if let Some((path, _)) = storage_prefixes
            .iter()
            .find(|(_, prefix)| is_prefix_of(prefix, k))
        {
            added_per_path_cnt.bump(path);
            state.insert(k.clone(), v.clone());
        }
    }

    for (path, prefix) in storage_prefixes {
        info!(
            "For storage path `{}` (prefix `{}`) Replaced {} entries by {} entries from initial_spec",
            path.0, prefix, removed_per_path_count[&path], added_per_path_cnt[&path]
        );
    }
    state
}
