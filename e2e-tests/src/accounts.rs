use aleph_client::{keypair_from_string, KeyPair};

use crate::config::Config;

// this should be extracted to common code
pub fn get_validators_seeds(config: &Config) -> Vec<String> {
    match config.validators_seeds {
        Some(ref seeds) => seeds.clone(),
        None => (0..config.validators_count)
            .map(|seed| format!("//{}", seed))
            .collect(),
    }
}

pub fn get_validators_keys(config: &Config) -> Vec<KeyPair> {
    accounts_seeds_to_keys(&get_validators_seeds(config))
}

pub fn accounts_seeds_to_keys(seeds: &[String]) -> Vec<KeyPair> {
    seeds
        .iter()
        .map(String::as_str)
        .map(keypair_from_string)
        .collect()
}

pub fn get_sudo_key(config: &Config) -> KeyPair {
    keypair_from_string(&config.sudo_seed)
}
