use aleph_client::{create_connection, from as parse_to_protocol, KeyPair, Protocol};
use clap::{Parser, Subcommand};
use sp_core::Pair;
use std::env;

use cliain::{change_validators, prepare_keys};

#[derive(Debug, Parser, Clone)]
#[clap(version = "1.0")]
struct Config {
    /// WS endpoint address of the node to connect to
    #[clap(long, default_value = "127.0.0.1:9944")]
    pub node: String,

    /// Protocol to be used for connecting to node (`ws` or `wss`)
    #[clap(name = "use_ssl", parse(from_flag = parse_to_protocol))]
    pub protocol: Protocol,

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

    let Config {
        node,
        protocol,
        seed,
        command,
    } = Config::parse();
    let key = KeyPair::from_string(&seed, None).expect("Can't create pair from seed value");
    let connection = create_connection(node.as_str(), protocol).set_signer(key.clone());
    match command {
        Command::ChangeValidators { validators } => change_validators(connection, validators),
        Command::PrepareKeys => prepare_keys(connection, key),
    }
}

fn init_env() {
    if env::var(env_logger::DEFAULT_FILTER_ENV).is_err() {
        env::set_var(env_logger::DEFAULT_FILTER_ENV, "info");
    }
    env_logger::init();
}
