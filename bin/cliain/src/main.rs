use aleph_client::{keypair_from_string, print_storages, BlockNumber, SignedConnection};
use clap::{Parser, Subcommand};
use log::{error, info};
use sp_core::Pair;
use std::env;
use substrate_api_client::AccountId;

use cliain::{
    bond, change_validators, force_new_era, nominate, prepare_keys, prompt_password_hidden,
    rotate_keys, set_keys, set_staking_limits, transfer, update_runtime, validate, vest,
    vest_other, vested_transfer, ConnectionConfig,
};
use primitives::Balance;

#[derive(Debug, Parser, Clone)]
#[clap(version = "1.0")]
struct Config {
    /// WS endpoint address of the node to connect to
    #[clap(long, default_value = "127.0.0.1:9944")]
    pub node: String,

    /// The seed of the key to use for signing calls
    /// If not given, a user is prompted to provide seed
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

    /// Declare the desire to nominate target account
    Nominate {
        #[clap(long)]
        nominee: String,
    },

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

        /// Maximum number of nominators
        #[clap(long)]
        max_nominators_count: Option<u32>,

        /// Maximum number of validators
        #[clap(long)]
        max_validators_count: Option<u32>,
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

    /// Update vesting for the calling account.
    Vest,

    /// Update vesting on behalf of the given account.
    VestOther {
        /// Account seed for which vesting should be performed.
        #[clap(long)]
        vesting_account: String,
    },

    /// Transfer funds via balances pallet
    VestedTransfer {
        /// Number of tokens to send.
        #[clap(long)]
        amount_in_tokens: u64,

        /// Seed of the target account.
        #[clap(long)]
        to_account: String,

        /// How much balance (in rappens, not in tokens) should be unlocked per block.
        #[clap(long)]
        per_block: Balance,

        /// Block number when unlocking should start.
        #[clap(long)]
        starting_block: BlockNumber,
    },

    /// Print debug info of storage
    DebugStorage,
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
            let key = keypair_from_string(&seed);
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
        Command::RotateKeys => rotate_keys::<SignedConnection>(cfg.into()),
        Command::SetStakingLimits {
            minimal_nominator_stake,
            minimal_validator_stake,
            max_nominators_count,
            max_validators_count,
        } => set_staking_limits(
            cfg.into(),
            minimal_nominator_stake,
            minimal_validator_stake,
            max_nominators_count,
            max_validators_count,
        ),
        Command::ForceNewEra => {
            force_new_era(cfg.into());
        }
        Command::SeedToSS58 => info!(
            "SS58 Address: {}",
            keypair_from_string(&seed).public().to_string()
        ),
        Command::DebugStorage => print_storages::<SignedConnection>(&cfg.into()),
        Command::UpdateRuntime { runtime } => update_runtime(cfg.into(), runtime),
        Command::Vest => vest(cfg.into()),
        Command::VestOther { vesting_account } => vest_other(cfg.into(), vesting_account),
        Command::VestedTransfer {
            to_account,
            amount_in_tokens,
            per_block,
            starting_block,
        } => vested_transfer(
            cfg.into(),
            to_account,
            amount_in_tokens,
            per_block,
            starting_block,
        ),
        Command::Nominate { nominee } => nominate(cfg.into(), nominee),
    }
}

fn init_env() {
    if env::var(env_logger::DEFAULT_FILTER_ENV).is_err() {
        env::set_var(env_logger::DEFAULT_FILTER_ENV, "info");
    }
    env_logger::init();
}
