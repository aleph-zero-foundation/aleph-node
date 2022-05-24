use std::{collections::HashMap, ops::Index};

use log::info;

use crate::{
    types::{Get, StorageKey, StoragePath},
    Storage,
};

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
        self.map.get(path).unwrap_or(&0)
    }
}

pub fn combine_states(
    mut state: Storage,
    initial_state: Storage,
    storage_to_keep: Vec<StoragePath>,
) -> Storage {
    let storage_prefixes = storage_to_keep
        .into_iter()
        .map(|path| (path.clone(), path.into()))
        .collect::<Vec<(StoragePath, StorageKey)>>();

    let mut removed_per_path_count = PathCounter::default();
    let mut added_per_path_cnt = PathCounter::default();

    state.retain(|k, _v| {
        match storage_prefixes
            .iter()
            .find(|(_, prefix)| prefix.is_prefix_of(k))
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
            .find(|(_, prefix)| prefix.is_prefix_of(k))
        {
            added_per_path_cnt.bump(path);
            state.insert(k.clone(), v.clone());
        }
    }

    for (path, prefix) in storage_prefixes {
        info!(
            "For storage path `{}` (prefix `{}`) Replaced {} entries by {} entries from initial_spec",
            path.clone().get(), prefix.clone().get(), removed_per_path_count[&path], added_per_path_cnt[&path]
        );
    }
    state
}
