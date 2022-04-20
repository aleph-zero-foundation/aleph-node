use clap::{ErrorKind, Parser, CommandFactory};
use log::{info, trace, warn};
use rand::{thread_rng, Rng};
use rayon::prelude::*;
use sp_core::{sr25519::Pair as KeyPair, Pair};
use sp_keyring::AccountKeyring;
use std::iter;
use substrate_api_client::{extrinsic::staking::RewardDestination, AccountId, XtStatus};

use aleph_client::{
    balances_batch_transfer, create_connection, keypair_from_string,
    payout_stakers_and_assert_locked_balance, staking_batch_bond, staking_batch_nominate,
    staking_bond, staking_validate, wait_for_next_era, Connection,
};
use primitives::{
    staking::{MAX_NOMINATORS_REWARDED_PER_VALIDATOR, MIN_NOMINATOR_BOND, MIN_VALIDATOR_BOND},
    TOKEN,
};
use sp_core::crypto::AccountId32;

// testcase parameters
const NOMINATOR_COUNT: u32 = MAX_NOMINATORS_REWARDED_PER_VALIDATOR;
const ERAS_TO_WAIT: u32 = 100;

// we need to schedule batches for limited call count, otherwise we'll exhaust a block max weight
const BOND_CALL_BATCH_LIMIT: usize = 256;
const NOMINATE_CALL_BATCH_LIMIT: usize = 220;
const TRANSFER_CALL_BATCH_LIMIT: usize = 1024;

#[derive(Debug, Parser, Clone)]
#[clap(version = "1.0")]
struct Config {
    /// WS endpoint address of the node to connect to. Use IP:port syntax, e.g. 127.0.0.1:9944
    #[clap(long, default_value = "127.0.0.1:9944")]
    pub address: String,

    /// A path to a file that contains the root account seed.
    /// If not given, Alice is assumed to be the root.
    #[clap(long)]
    pub root_seed_file: Option<String>,

    /// A path to a file that contains seeds of validators, each in a separate line.
    /// If not given, validators 0, 1, ..., validator_count are assumed
    #[clap(long)]
    pub validators_seed_file: Option<String>,

    /// number of testcase validators
    #[clap(long)]
    pub validator_count: Option<u32>,
}

fn main() -> Result<(), anyhow::Error> {
    let Config {
        address,
        root_seed_file,
        validators_seed_file,
        validator_count,
    } = Config::parse();

    if !(validator_count.is_some() ^ validators_seed_file.is_some()) {
        let mut cmd = Config::command();
        cmd.error(
            ErrorKind::ArgumentConflict,
            "only one of --validator-count or --validators-seed-file must be specified!",
        ).exit();
    }

    let sudoer = match root_seed_file {
        Some(root_seed_file) => {
            let root_seed = std::fs::read_to_string(&root_seed_file)
                .expect(&format!("Failed to read file {}", root_seed_file));
            keypair_from_string(root_seed.trim())
        }
        None => AccountKeyring::Alice.pair(),
    };

    env_logger::init();

    let connection = create_connection(&address).set_signer(sudoer);
    let validators = match validators_seed_file {
        Some(validators_seed_file) => {
            let validators_seeds = std::fs::read_to_string(&validators_seed_file)
                .expect(&format!("Failed to read file {}", validators_seed_file));
            validators_seeds
                .split("\n")
                .filter(|seed| !seed.is_empty())
                .map(keypair_from_string)
                .collect()
        }
        None => (0..validator_count.unwrap())
            .map(|validator_number| derive_user_account_from_numeric_seed(validator_number))
            .collect::<Vec<_>>(),
    };
    let validator_count = validators.len() as u32;
    warn!("Make sure you have exactly {} nodes run in the background, otherwise you'll see extrinsic send failed errors.",validator_count);

    let validators = bond_validate(&address, validators);
    let validators_and_its_nominators =
        create_test_validators_and_its_nominators(&connection, validators, validator_count);
    wait_for_successive_eras(
        &address,
        &connection,
        validators_and_its_nominators,
        ERAS_TO_WAIT,
    )?;

    Ok(())
}

fn create_test_validators_and_its_nominators(
    connection: &Connection,
    validators: Vec<KeyPair>,
    validators_count: u32,
) -> Vec<(KeyPair, Vec<AccountId32>)> {
    validators
        .iter()
        .enumerate()
        .map(|(validator_index, validator_pair)| {
            let nominator_accounts = generate_nominator_accounts_with_minimal_bond(
                &connection,
                validator_index as u32,
                validators_count,
            );
            let nominee_account = AccountId::from(validator_pair.public());
            info!("Nominating validator {}", nominee_account);
            nominate_validator(&connection, nominator_accounts.clone(), nominee_account);
            (validator_pair.clone(), nominator_accounts)
        })
        .collect()
}

pub fn derive_user_account_from_numeric_seed(seed: u32) -> KeyPair {
    trace!("Generating account from numeric seed {}", seed);
    keypair_from_string(&format!("//{}", seed))
}

fn wait_for_successive_eras(
    address: &str,
    connection: &Connection,
    validators_and_its_nominators: Vec<(KeyPair, Vec<AccountId>)>,
    eras_to_wait: u32,
) -> Result<(), anyhow::Error> {
    // in order to have over 8k nominators we need to wait around 60 seconds all calls to be processed
    // that means not all 8k nominators we'll make i to era 1st, hence we need to wait to 2nd era
    wait_for_next_era(connection)?;
    wait_for_next_era(connection)?;
    // then we wait another full era to test rewards
    let mut current_era = wait_for_next_era(connection)?;
    for _ in 0..eras_to_wait {
        info!(
            "Era {} started, claiming rewards for era {}",
            current_era,
            current_era - 1
        );
        validators_and_its_nominators
            .iter()
            .for_each(|(validator, nominators)| {
                let stash_connection = create_connection(address).set_signer(validator.clone());
                let stash_account = AccountId::from(validator.public());
                info!("Doing payout_stakers for validator {}", stash_account);
                payout_stakers_and_assert_locked_balance(
                    &stash_connection,
                    &[&nominators[..], &[stash_account.clone()]].concat(),
                    &stash_account,
                    current_era,
                );
            });
        current_era = wait_for_next_era(connection)?;
    }
    Ok(())
}

fn nominate_validator(
    connection: &Connection,
    nominator_accounts: Vec<AccountId>,
    nominee_account: AccountId,
) {
    let stash_validators_accounts = nominator_accounts
        .iter()
        .zip(nominator_accounts.iter())
        .collect::<Vec<_>>();
    stash_validators_accounts
        .chunks(BOND_CALL_BATCH_LIMIT)
        .for_each(|chunk| {
            let mut rng = thread_rng();
            staking_batch_bond(
                connection,
                chunk,
                (rng.gen::<u128>() % 100) * TOKEN + MIN_NOMINATOR_BOND,
                RewardDestination::Staked,
            )
        });
    let nominator_nominee_accounts = nominator_accounts
        .iter()
        .zip(iter::repeat(&nominee_account))
        .collect::<Vec<_>>();
    nominator_nominee_accounts
        .chunks(NOMINATE_CALL_BATCH_LIMIT)
        .for_each(|chunk| staking_batch_nominate(connection, chunk));
}

fn bond_validate(address: &str, validators: Vec<KeyPair>) -> Vec<KeyPair> {
    validators.par_iter().for_each(|account| {
        let connection = create_connection(address).set_signer(account.clone());
        let controller_account_id = AccountId::from(account.public());
        staking_bond(
            &connection,
            MIN_VALIDATOR_BOND,
            &controller_account_id,
            XtStatus::InBlock,
        );
    });
    validators.par_iter().for_each(|account| {
        let mut rng = thread_rng();
        let connection = create_connection(address).set_signer(account.clone());
        staking_validate(&connection, rng.gen::<u8>() % 100, XtStatus::InBlock);
    });
    validators
}

fn generate_nominator_accounts_with_minimal_bond(
    connection: &Connection,
    validator_number: u32,
    validators_count: u32,
) -> Vec<AccountId> {
    info!(
        "Generating nominator accounts for validator {}",
        validator_number
    );
    let accounts = (0..NOMINATOR_COUNT)
        .map(|nominator_number| {
            derive_user_account_from_numeric_seed(
                validators_count + nominator_number + NOMINATOR_COUNT * validator_number,
            )
        })
        .map(|key_pair| AccountId::from(key_pair.public()))
        .collect::<Vec<_>>();
    accounts
        .chunks(TRANSFER_CALL_BATCH_LIMIT)
        .for_each(|chunk| {
            balances_batch_transfer(connection, chunk.to_vec(), MIN_NOMINATOR_BOND * 10);
        });
    accounts
}
