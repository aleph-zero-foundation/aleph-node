use std::env;

use aleph_client::{RootConnection, SignedConnection};
use clap::{Args, Parser};
use once_cell::sync::Lazy;
use primitives::SessionIndex;

use crate::accounts::{get_sudo_key, get_validators_keys, get_validators_seeds, NodeKeys};

static GLOBAL_CONFIG: Lazy<Config> = Lazy::new(|| {
    let unparsed = env::var("E2E_CONFIG").unwrap_or("".to_string());
    let unparsed = format!("e2e {}", unparsed);
    Config::parse_from(unparsed.split_whitespace())
});

pub fn setup_test() -> &'static Config {
    let _ = env_logger::builder().is_test(true).try_init();
    &GLOBAL_CONFIG
}

#[derive(Debug, Parser, Clone)]
#[clap(version = "1.0")]
pub struct Config {
    /// WS endpoint address of the node to connect to
    #[clap(long, default_value = "ws://127.0.0.1:9943")]
    pub node: String,

    /// Test cases to run.
    #[clap(long)]
    pub test_cases: Option<Vec<String>>,

    /// Number of //0, //1, ... validators to run e2e tests on
    #[clap(long, default_value = "5")]
    pub validator_count: u32,

    /// Seed values to create accounts
    /// Optional: by default we use //0, //1, ... seeds for validators
    #[clap(long)]
    pub validators_seeds: Option<Vec<String>>,

    /// Seed value of sudo account
    #[clap(long, default_value = "//Alice")]
    pub sudo_seed: String,

    /// Test case parameters, used for test setup.
    #[clap(flatten)]
    pub test_case_params: TestCaseParams,
}

impl Config {
    /// Returns keys associated with the node represented by this Config (first of the validators_seeds).
    /// Panics if Config is invalid.
    pub fn node_keys(&self) -> NodeKeys {
        let validator_seed = get_validators_seeds(self)
            .into_iter()
            .next()
            .expect("We should have a seed for at least one validator");
        NodeKeys::from(validator_seed)
    }

    pub async fn create_root_connection(&self) -> RootConnection {
        let sudo_keypair = get_sudo_key(self);
        RootConnection::new(self.node.clone(), sudo_keypair)
            .await
            .unwrap()
    }

    /// Get a `SignedConnection` where the signer is the first validator.
    pub async fn get_first_signed_connection(&self) -> SignedConnection {
        let node = &self.node;
        let mut accounts = get_validators_keys(self);
        let sender = accounts.remove(0);
        SignedConnection::new(node.clone(), sender).await
    }
}

/// Parameters which can be passed to test cases.
#[derive(Args, Clone, Debug)]
pub struct TestCaseParams {
    /// Desired number of reserved seats for validators, may be set within the test.
    #[clap(long)]
    pub reserved_seats: Option<u32>,

    /// Desired number of non-reserved seats for validators, may be set within the test.
    #[clap(long)]
    pub non_reserved_seats: Option<u32>,

    /// Version for the VersionUpgrade test.
    #[clap(long)]
    pub upgrade_to_version: Option<u32>,

    /// Session in which we should schedule an upgrade in VersionUpgrade test.
    #[clap(long)]
    pub upgrade_session: Option<SessionIndex>,

    /// How many sessions we should wait after upgrade in VersionUpgrade test.
    #[clap(long)]
    pub upgrade_finalization_wait_sessions: Option<u32>,
}
