use clap::Parser;

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
