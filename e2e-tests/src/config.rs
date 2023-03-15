use std::{env, str::FromStr};

use aleph_client::{RootConnection, SignedConnection};
use once_cell::sync::Lazy;
use primitives::SessionIndex;

use crate::accounts::{get_sudo_key, get_validators_keys, get_validators_seeds, NodeKeys};

static GLOBAL_CONFIG: Lazy<Config> = Lazy::new(|| {
    let node = get_env("NODE_URL").unwrap_or_else(|| "ws://127.0.0.1:9944".to_string());
    let validator_count = get_env("VALIDATOR_COUNT").unwrap_or(5);
    let validators_seeds = env::var("VALIDATORS_SEEDS")
        .ok()
        .map(|s| s.split(',').map(|s| s.to_string()).collect());
    let sudo_seed = get_env("SUDO_SEED").unwrap_or_else(|| "//Alice".to_string());

    Config {
        node,
        validator_count,
        validators_seeds,
        sudo_seed,
        test_case_params: TestCaseParams {
            reserved_seats: get_env("RESERVED_SEATS"),
            non_reserved_seats: get_env("NON_RESERVED_SEATS"),
            upgrade_to_version: get_env("UPGRADE_VERSION"),
            upgrade_session: get_env("UPGRADE_SESSION"),
            upgrade_finalization_wait_sessions: get_env("UPGRADE_FINALIZATION_WAIT_SESSIONS"),
            adder: get_env("ADDER"),
            adder_metadata: get_env("ADDER_METADATA"),
            back_to_the_future: get_env("BACK_TO_THE_FUTURE"),
            early_bird_special: get_env("EARLY_BIRD_SPECIAL"),
            the_pressiah_cometh: get_env("THE_PRESSIAH_COMETH"),
            wrapped_azero: get_env("WRAPPED_AZERO"),
            simple_dex: get_env("SIMPLE_DEX"),
            button_game_metadata: get_env("BUTTON_GAME_METADATA"),
            marketplace_metadata: get_env("MARKETPLACE_METADATA"),
            reward_token_metadata: get_env("REWARD_TOKEN_METADATA"),
            ticket_token_metadata: get_env("TICKET_TOKEN_METADATA"),
            simple_dex_metadata: get_env("SIMPLE_DEX_METADATA"),
            wrapped_azero_metadata: get_env("WRAPPED_AZERO_METADATA"),
            out_latency: get_env("OUT_LATENCY"),
            synthetic_network_urls: env::var("SYNTHETIC_URLS")
                .ok()
                .map(|s| s.split(',').map(|s| s.to_string()).collect()),
        },
    }
});

fn get_env<T>(name: &str) -> Option<T>
where
    T: FromStr,
    T::Err: std::fmt::Debug,
{
    env::var(name).ok().map(|v| {
        v.parse()
            .unwrap_or_else(|_| panic!("Failed to parse env var {}", name))
    })
}

pub fn setup_test() -> &'static Config {
    let _ = env_logger::builder().is_test(true).try_init();
    &GLOBAL_CONFIG
}

#[derive(Debug, Clone)]
pub struct Config {
    /// WS endpoint address of the node to connect to
    pub node: String,

    /// Number of //0, //1, ... validators to run e2e tests on
    pub validator_count: u32,

    /// Seed values to create accounts
    /// Optional: by default we use //0, //1, ... seeds for validators
    pub validators_seeds: Option<Vec<String>>,

    /// Seed value of sudo account
    pub sudo_seed: String,

    /// Test case parameters, used for test setup.
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
        RootConnection::new(&self.node, sudo_keypair).await.unwrap()
    }

    pub fn validator_names(&self) -> Vec<String> {
        (0..self.validator_count)
            .map(|id| format!("Node{}", id))
            .collect()
    }

    pub fn synthetic_network_urls(&self) -> Vec<String> {
        match &self.test_case_params.synthetic_network_urls {
            Some(urls) => urls.clone(),
            None => self
                .validator_names()
                .into_iter()
                .map(|node_name| format!("http://{}:80/qos", node_name))
                .collect(),
        }
    }

    /// Get a `SignedConnection` where the signer is the first validator.
    pub async fn get_first_signed_connection(&self) -> SignedConnection {
        let node = &self.node;
        let mut accounts = get_validators_keys(self);
        let sender = accounts.remove(0);
        SignedConnection::new(node, sender).await
    }

    pub async fn create_signed_connections(&self) -> Vec<SignedConnection> {
        futures::future::join_all(
            get_validators_keys(self)
                .into_iter()
                .map(|account| async { SignedConnection::new(&self.node, account).await }),
        )
        .await
    }
}

/// Parameters which can be passed to test cases.
#[derive(Clone, Debug)]
pub struct TestCaseParams {
    /// Desired number of reserved seats for validators, may be set within the test.
    pub reserved_seats: Option<u32>,

    /// Desired number of non-reserved seats for validators, may be set within the test.
    pub non_reserved_seats: Option<u32>,

    /// Address of the Early Bird Special game contract, only used by button game tests.
    pub early_bird_special: Option<String>,

    /// Address of the Back to the Future game contract, only used by button game tests.
    pub back_to_the_future: Option<String>,

    /// Address of the The Pressiah Cometh game contract, only used by button game tests.
    pub the_pressiah_cometh: Option<String>,

    /// Address of the simple dex contract. Only used by button tests.
    pub simple_dex: Option<String>,

    /// Address of the wrapped azero contract. Only used by button tests.
    pub wrapped_azero: Option<String>,

    /// Path to the button game metadata file. Only used by button tests.
    pub button_game_metadata: Option<String>,

    /// Path to the ticket token metadata file. Only used by button tests.
    pub ticket_token_metadata: Option<String>,

    /// Path to the reward token metadata file. Only used by button tests.
    pub reward_token_metadata: Option<String>,

    /// Path to the marketplace metadata file. Only used by button tests.
    pub marketplace_metadata: Option<String>,

    /// Path to the simple_dex metadata file. Only used by button tests.
    pub simple_dex_metadata: Option<String>,

    /// Path to wrapped_azero metadata file. Only used by button tests.
    pub wrapped_azero_metadata: Option<String>,

    /// Version for the VersionUpgrade test.
    pub upgrade_to_version: Option<u32>,

    /// Session in which we should schedule an upgrade in VersionUpgrade test.
    pub upgrade_session: Option<SessionIndex>,

    /// How many sessions we should wait after upgrade in VersionUpgrade test.
    pub upgrade_finalization_wait_sessions: Option<u32>,

    /// Adder contract address.
    pub adder: Option<String>,

    /// Adder contract metadata.
    pub adder_metadata: Option<String>,

    /// Milliseconds of network latency
    pub out_latency: Option<u64>,

    /// List of URLs for the configuration endpoints of the synthetic-network
    pub synthetic_network_urls: Option<Vec<String>>,
}
