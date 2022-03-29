use aleph_client::{keypair_from_string, KeyPair};

use crate::config::Config;

pub fn default_account_seeds() -> Vec<String> {
    ["//Damian", "//Hansu", "//Tomasz", "//Zbyszko", "//Julia"]
        .into_iter()
        .map(String::from)
        .collect()
}

pub fn accounts_from_seeds(seeds: &Option<Vec<String>>) -> Vec<KeyPair> {
    match seeds {
        Some(ref seeds) => seeds.clone(),
        None => default_account_seeds(),
    }
    .iter()
    .map(String::as_str)
    .map(keypair_from_string)
    .collect()
}

pub fn get_sudo(config: &Config) -> KeyPair {
    match &config.sudo {
        Some(seed) => keypair_from_string(seed),
        None => keypair_from_string(&*default_account_seeds()[0]),
    }
}
