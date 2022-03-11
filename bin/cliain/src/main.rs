use std::env;
use clap::{Parser, Subcommand};
use aleph_client::KeyPair;
use sp_core::Pair;

use cliain::{change_validators, prepare_keys};

#[derive(Debug, Parser, Clone)]
#[clap(version = "1.0")]
struct Config {
    /// WS endpoint address of the node to connect to
    #[clap(long, default_value = "127.0.0.1:9944")]
    pub node: String,

    /// The seed of the key to use for signing calls
    #[clap(long)]
    pub seed: String,

    /// Specific command to execute
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, Subcommand)]
enum Command {
    /// Change the validator set for the session after the next
    ChangeValidators {
        /// The new validators
        #[clap(long, value_delimiter = ',')]
        validators: Vec<String>,
    },
    /// Associate the node with a specific staking account.
    PrepareKeys,
}

fn main() {
    init_env();

    let Config{
        node,
        seed,
        command,
    } = Config::parse();
    let key = KeyPair::from_string(&seed, None).expect("Can't create pair from seed value");
    match command {
        Command::ChangeValidators{validators} => change_validators(validators, node, key),
        Command::PrepareKeys => prepare_keys(node, key),
    }
}

fn init_env() {
    if env::var(env_logger::DEFAULT_FILTER_ENV).is_err() {
        env::set_var(env_logger::DEFAULT_FILTER_ENV, "info");
    }
    env_logger::init();
}
