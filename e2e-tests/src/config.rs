use aleph_client::RootConnection;
use clap::Parser;

use crate::accounts::{get_sudo_key, get_validators_seeds, NodeKeys};

#[derive(Debug, Parser, Clone)]
#[clap(version = "1.0")]
pub struct Config {
    /// WS endpoint address of the node to connect to
    #[clap(long, default_value = "127.0.0.1:9943")]
    pub node: String,

    /// Test cases to run.
    #[clap(long)]
    pub test_cases: Option<Vec<String>>,

    /// Number of //0, //1, ... validators to run e2e tests on
    #[clap(long, default_value = "5")]
    pub validators_count: u32,

    /// seed values to create accounts
    /// Optional: by default we use //0, //1, ... seeds for validators
    #[clap(long)]
    pub validators_seeds: Option<Vec<String>>,

    /// seed value of sudo account
    #[clap(long, default_value = "//Alice")]
    pub sudo_seed: String,
}

impl Config {
    /// Returns keys associated with the node represented by this Config (first of the validators_seeds).
    /// Panics if Config is invalid.
    pub fn node_keys(&self) -> NodeKeys {
        let validator_seed = get_validators_seeds(self)
            .into_iter()
            .next()
            .expect("we should have a seed for at least one validator");
        NodeKeys::from(validator_seed)
    }

    pub fn create_root_connection(&self) -> RootConnection {
        let sudo_keypair = get_sudo_key(self);
        RootConnection::new(&self.node, sudo_keypair)
    }
}
