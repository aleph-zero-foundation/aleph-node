use aleph_client::KeyPair;
use clap::{Parser, Subcommand};
use log::{error, info};
use sp_core::Pair;
use std::env;
use substrate_api_client::AccountId;

use cliain::{
    bond, change_validators, force_new_era, prepare_keys, prompt_password_hidden, rotate_keys,
    set_keys, set_staking_limits, transfer, update_runtime, validate, ConnectionConfig,
};

#[derive(Debug, Parser, Clone)]
#[clap(version = "1.0")]
struct Config {
    /// WS endpoint address of the node to connect to
    #[clap(long, default_value = "127.0.0.1:9944")]
    pub node: String,

    /// The seed of the key to use for signing calls
    /// If not given, an user is prompted to provide seed
    #[clap(long)]
    pub seed: Option<String>,

    /// Specific command that executes either a signed transaction or is an auxiliary command
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, Subcommand)]
enum Command {
    /// Staking call to bond stash with controller
    Bond {
        /// SS58 id of the controller account
        #[clap(long)]
        controller_account: String,

        /// a Stake to bond (in tokens)
        #[clap(long)]
        initial_stake_tokens: u32,
    },

    /// Change the validator set for the session after the next
    ChangeValidators {
        /// The new validators
        #[clap(long, value_delimiter = ',')]
        validators: Vec<String>,
    },

    /// Force new era in staking world. Requires sudo.
    ForceNewEra,

    /// Associate the node with a specific staking account.
    PrepareKeys,

    /// Call rotate_keys() RPC call and prints them to stdout
    RotateKeys,

    /// Sets given keys for origin controller
    SetKeys {
        /// 64 byte hex encoded string in form 0xaabbcc..
        /// where aabbcc...  must be exactly 128 characters long
        #[clap(long)]
        new_keys: String,
    },

    /// Command to convert given seed to SS58 Account id
    SeedToSS58,

    /// Sets lower bound for nominator and validator. Requires root account.
    SetStakingLimits {
        /// Nominator lower bound
        #[clap(long)]
        minimal_nominator_stake: u64,

        /// Validator lower bound
        #[clap(long)]
        minimal_validator_stake: u64,
    },

    /// Transfer funds via balances pallet
    Transfer {
        /// Number of tokens to send,
        #[clap(long)]
        amount_in_tokens: u64,

        /// SS58 id of target account
        #[clap(long)]
        to_account: String,
    },

    /// Send new runtime (requires sudo account)
    UpdateRuntime {
        #[clap(long)]
        /// Path to WASM file with runtime
        runtime: String,
    },

    /// Call staking validate call for a given controller
    Validate {
        /// Validator commission percentage
        #[clap(long)]
        commission_percentage: u8,
    },
}

fn main() {
    init_env();

    let Config {
        node,
        seed,
        command,
    } = Config::parse();

    let seed = match seed {
        Some(seed) => seed,
        None => match prompt_password_hidden("Provide seed for the signer account:") {
            Ok(seed) => seed,
            Err(e) => {
                error!("Failed to parse prompt with error {:?}! Exiting.", e);
                std::process::exit(1);
            }
        },
    };
    let cfg = ConnectionConfig::new(node, seed.clone());
    match command {
        Command::ChangeValidators { validators } => change_validators(cfg.into(), validators),
        Command::PrepareKeys => {
            let key = KeyPair::from_string(&seed, None).expect("Can't create pair from seed value");
            let controller_account_id = AccountId::from(key.public());
            prepare_keys(cfg.into(), controller_account_id);
        }
        Command::Bond {
            controller_account,
            initial_stake_tokens,
        } => bond(cfg.into(), initial_stake_tokens, controller_account),
        Command::SetKeys { new_keys } => set_keys(cfg.into(), new_keys),
        Command::Validate {
            commission_percentage,
        } => validate(cfg.into(), commission_percentage),
        Command::Transfer {
            amount_in_tokens,
            to_account,
        } => transfer(cfg.into(), amount_in_tokens, to_account),
        Command::RotateKeys => rotate_keys(cfg.into()),
        Command::SetStakingLimits {
            minimal_nominator_stake,
            minimal_validator_stake,
        } => set_staking_limits(cfg.into(), minimal_nominator_stake, minimal_validator_stake),
        Command::ForceNewEra => {
            force_new_era(cfg.into());
        }
        Command::SeedToSS58 => info!(
            "SS58 Address: {}",
            KeyPair::from_string(&seed, None)
                .expect("Can't create pair from seed value")
                .public()
                .to_string()
        ),
        Command::UpdateRuntime { runtime } => update_runtime(cfg.into(), runtime),
    }
}

fn init_env() {
    if env::var(env_logger::DEFAULT_FILTER_ENV).is_err() {
        env::set_var(env_logger::DEFAULT_FILTER_ENV, "info");
    }
    env_logger::init();
}
