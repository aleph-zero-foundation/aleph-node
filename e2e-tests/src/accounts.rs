use sp_core::Pair;
use substrate_api_client::sp_runtime::AccountId32;
use substrate_api_client::Balance;

use crate::config::Config;
use crate::{Connection, KeyPair};

pub fn keypair_from_string(seed: String) -> KeyPair {
    KeyPair::from_string(&seed, None).expect("Can't create pair from seed value")
}

pub fn accounts_from_seeds(seeds: Option<Vec<String>>) -> Vec<KeyPair> {
    let seeds = seeds.unwrap_or_else(|| {
        vec![
            "//Damian".into(),
            "//Tomasz".into(),
            "//Zbyszko".into(),
            "//Hansu".into(),
        ]
    });
    seeds.into_iter().map(keypair_from_string).collect()
}

pub fn get_sudo(config: Config) -> KeyPair {
    match config.sudo {
        Some(seed) => keypair_from_string(seed),
        None => accounts_from_seeds(config.seeds)[0].to_owned(),
    }
}

pub fn get_free_balance(account: &AccountId32, connection: &Connection) -> Balance {
    connection.get_account_data(account).unwrap().unwrap().free
}
