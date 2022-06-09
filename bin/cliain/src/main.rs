use aleph_client::{keypair_from_string, print_storages, SignedConnection};
use clap::Parser;
use cliain::{
    bond, call, change_validators, force_new_era, instantiate, instantiate_with_code, nominate,
    prepare_keys, prompt_password_hidden, remove_code, rotate_keys, set_keys, set_staking_limits,
    transfer, update_runtime, upload_code, validate, vest, vest_other, vested_transfer, Command,
    ConnectionConfig,
};
use log::{error, info};
use sp_core::Pair;
use std::env;
use substrate_api_client::AccountId;

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
        Command::ContractInstantiateWithCode(command) => {
            match instantiate_with_code(cfg.into(), command) {
                Ok(result) => println!(
                    "{}",
                    serde_json::to_string(&result).expect("Can't encode the result as JSON")
                ),
                Err(why) => error!("Contract deployment failed {:?}", why),
            };
        }
        Command::ContractUploadCode(command) => match upload_code(cfg.into(), command) {
            Ok(result) => println!("{:?}", result),
            Err(why) => error!("Contract upload failed {:?}", why),
        },
        Command::ContractCall(command) => match call(cfg.into(), command) {
            Ok(result) => println!("{:?}", result),
            Err(why) => error!("Contract call failed {:?}", why),
        },
        Command::ContractInstantiate(command) => match instantiate(cfg.into(), command) {
            Ok(result) => println!("{:?}", result),
            Err(why) => error!("Contract instantiate failed {:?}", why),
        },
        Command::ContractRemoveCode(command) => match remove_code(cfg.into(), command) {
            Ok(result) => println!("{:?}", result),
            Err(why) => error!("Contract remove code failed {:?}", why),
        },
    }
}

fn init_env() {
    if env::var(env_logger::DEFAULT_FILTER_ENV).is_err() {
        env::set_var(env_logger::DEFAULT_FILTER_ENV, "info");
    }
    env_logger::init();
}
