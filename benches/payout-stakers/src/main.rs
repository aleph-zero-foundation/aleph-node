use std::{iter, time::Instant};

use aleph_client::{
    keypair_from_string,
    pallets::{
        balances::{BalanceApi, BalanceUserBatchExtApi},
        staking::{StakingApi, StakingApiExt, StakingUserApi},
    },
    waiting::{BlockStatus, WaitingExt},
    AccountId, Balance, ConnectionApi, KeyPair, RootConnection, SignedConnection,
    SignedConnectionApi, TxStatus,
};
use clap::{ArgGroup, Parser};
use futures::future::join_all;
use log::{info, trace, warn};
use primitives::{
    staking::{MAX_NOMINATORS_REWARDED_PER_VALIDATOR, MIN_NOMINATOR_BOND, MIN_VALIDATOR_BOND},
    EraIndex, TOKEN,
};
use rand::{thread_rng, Rng};
use sp_keyring::AccountKeyring;

// testcase parameters
const NOMINATOR_COUNT: u32 = MAX_NOMINATORS_REWARDED_PER_VALIDATOR;
const ERAS_TO_WAIT: u32 = 100;

// we need to schedule batches for limited call count, otherwise we'll exhaust a block max weight
const BOND_CALL_BATCH_LIMIT: usize = 256;
const NOMINATE_CALL_BATCH_LIMIT: usize = 220;
const TRANSFER_CALL_BATCH_LIMIT: usize = 1024;

#[derive(Debug, Parser, Clone)]
#[clap(version = "1.0")]
#[clap(group(ArgGroup::new("valid").required(true)))]
struct Config {
    /// WS endpoint address of the node to connect to. Use IP:port syntax, e.g. 127.0.0.1:9944
    #[clap(long, default_value = "ws://127.0.0.1:9944")]
    pub address: String,

    /// A path to a file that contains the root account seed.
    /// If not given, Alice is assumed to be the root.
    #[clap(long)]
    pub root_seed_file: Option<String>,

    /// A path to a file that contains seeds of validators, each in a separate line.
    /// If not given, validators 0, 1, ..., validator_count are assumed
    /// Only valid if validator count is not provided.
    #[clap(long, group = "valid")]
    pub validators_seed_file: Option<String>,

    /// Number of testcase validators.
    /// Only valid if validator seed file is not provided.
    #[clap(long, group = "valid")]
    pub validator_count: Option<u32>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    info!("Running payout_stakers bench.");
    let start = Instant::now();

    let Config {
        address,
        root_seed_file,
        validators_seed_file,
        validator_count,
    } = Config::parse();

    let sudoer = get_sudoer_keypair(root_seed_file);

    let connection = RootConnection::new(&address, sudoer).await.unwrap();

    let validators = match validators_seed_file {
        Some(validators_seed_file) => {
            let validators_seeds = std::fs::read_to_string(&validators_seed_file)
                .unwrap_or_else(|_| panic!("Failed to read file {validators_seed_file}"));
            validators_seeds
                .split('\n')
                .filter(|seed| !seed.is_empty())
                .map(keypair_from_string)
                .collect()
        }
        None => (0..validator_count.unwrap())
            .map(derive_user_account_from_numeric_seed)
            .collect::<Vec<_>>(),
    };
    let validator_count = validators.len() as u32;
    warn!("Make sure you have exactly {} nodes run in the background, otherwise you'll see extrinsic send failed errors.", validator_count);

    let controllers = generate_controllers_for_validators(validator_count);

    bond_validators_funds_and_choose_controllers(
        &address,
        controllers
            .iter()
            .map(|k| KeyPair::new(k.signer().clone()))
            .collect(),
    )
    .await;
    send_validate_txs(&address, controllers).await;

    let validators_and_nominator_stashes =
        setup_test_validators_and_nominator_stashes(&connection, validators).await;

    wait_for_successive_eras(
        &address,
        &connection,
        validators_and_nominator_stashes,
        ERAS_TO_WAIT,
    )
    .await?;

    let elapsed = Instant::now().duration_since(start);
    println!("Ok! Elapsed time {}ms", elapsed.as_millis());

    Ok(())
}

/// Get key pair based on seed file or default when seed file is not provided.
fn get_sudoer_keypair(root_seed_file: Option<String>) -> KeyPair {
    match root_seed_file {
        Some(root_seed_file) => {
            let root_seed = std::fs::read_to_string(&root_seed_file)
                .unwrap_or_else(|_| panic!("Failed to read file {root_seed_file}"));
            keypair_from_string(root_seed.trim())
        }
        None => keypair_from_string(&AccountKeyring::Alice.to_seed()),
    }
}

/// For a given set of validators, generates key pairs for the corresponding controllers.
fn generate_controllers_for_validators(validator_count: u32) -> Vec<KeyPair> {
    (0..validator_count)
        .map(|seed| keypair_from_string(&format!("//{seed}//Controller")))
        .collect::<Vec<_>>()
}

/// For a given set of validators, generates nominator accounts (controllers and stashes).
/// Bonds nominator controllers to the corresponding nominator stashes.
async fn setup_test_validators_and_nominator_stashes(
    connection: &RootConnection,
    validators: Vec<KeyPair>,
) -> Vec<(KeyPair, Vec<AccountId>)> {
    let mut validators_stashes = vec![];
    let validators_len = validators.len();
    for (validator_index, validator) in validators.into_iter().enumerate() {
        let nominator_stash_accounts = generate_nominator_accounts_with_minimal_bond(
            connection,
            validator_index as u32,
            validators_len as u32,
        )
        .await;
        let nominee_account = validator.account_id().clone();
        info!("Nominating validator {}", nominee_account);
        nominate_validator(
            connection,
            nominator_stash_accounts.clone(),
            nominee_account,
        )
        .await;
        validators_stashes.push((
            KeyPair::new(validator.signer().clone()),
            nominator_stash_accounts,
        ));
    }

    validators_stashes
}

pub fn derive_user_account_from_numeric_seed(seed: u32) -> KeyPair {
    trace!("Generating account from numeric seed {}", seed);
    keypair_from_string(&format!("//{seed}"))
}

/// For a given number of eras, in each era check whether stash balances of a validator are locked.
async fn wait_for_successive_eras<C: ConnectionApi + WaitingExt + StakingApi>(
    address: &str,
    connection: &C,
    validators_and_nominator_stashes: Vec<(KeyPair, Vec<AccountId>)>,
    eras_to_wait: u32,
) -> anyhow::Result<()> {
    // in order to have over 8k nominators we need to wait around 60 seconds all calls to be processed
    // that means not all 8k nominators we'll make i to era 1st, hence we need to wait to 2nd era
    // then we wait another full era to test rewards
    connection.wait_for_n_eras(3, BlockStatus::Finalized).await;
    let mut current_era = connection.get_current_era(None).await;
    for _ in 0..eras_to_wait {
        info!(
            "Era {} started, claiming rewards for era {}",
            current_era,
            current_era - 1
        );
        for (validator, nominators_stashes) in validators_and_nominator_stashes.iter() {
            let validator_connection =
                SignedConnection::new(address, KeyPair::new(validator.signer().clone())).await;
            let validator_account = validator.account_id().clone();
            info!("Doing payout_stakers for validator {}", validator_account);
            payout_stakers_and_assert_locked_balance(
                &validator_connection,
                &[&nominators_stashes[..], &[validator_account.clone()]].concat(),
                &validator_account,
                current_era,
            )
            .await;
        }
        connection.wait_for_n_eras(1, BlockStatus::Finalized).await;
        current_era = connection.get_current_era(None).await;
    }
    Ok(())
}

/// Nominates a specific validator based on the nominator controller and stash accounts.
async fn nominate_validator(
    connection: &RootConnection,
    nominator_stash_accounts: Vec<AccountId>,
    nominee_account: AccountId,
) {
    let mut rng = thread_rng();
    for chunk in nominator_stash_accounts
        .clone()
        .chunks(BOND_CALL_BATCH_LIMIT)
        .map(|c| c.to_vec())
    {
        let stake = (rng.gen::<Balance>() % 100) * TOKEN + MIN_NOMINATOR_BOND;
        connection
            .batch_bond(&chunk, stake, TxStatus::Submitted)
            .await
            .unwrap();
    }

    let nominator_nominee_accounts = nominator_stash_accounts
        .iter()
        .cloned()
        .zip(iter::repeat(&nominee_account).cloned())
        .collect::<Vec<_>>();
    for chunks in nominator_nominee_accounts.chunks(NOMINATE_CALL_BATCH_LIMIT) {
        connection
            .batch_nominate(chunks, TxStatus::InBlock)
            .await
            .unwrap();
    }
}

/// Bonds the funds of the validators.
/// Chooses controller accounts for the corresponding validators.
/// We assume stash == validator != controller.
async fn bond_validators_funds_and_choose_controllers(address: &str, validators: Vec<KeyPair>) {
    let mut handles = vec![];
    for validator in validators {
        let validator_address = address.to_string();
        handles.push(tokio::spawn(async move {
            let connection = SignedConnection::new(&validator_address, validator).await;
            connection
                .bond(MIN_VALIDATOR_BOND, TxStatus::InBlock)
                .await
                .unwrap();
        }));
    }
    join_all(handles).await;
}

/// Submits candidate validators via controller accounts.
/// We assume stash == validator != controller.
async fn send_validate_txs(address: &str, controllers: Vec<KeyPair>) {
    let mut handles = vec![];
    for controller in controllers {
        let node_address = address.to_string();
        let mut rng = thread_rng();
        let prc = rng.gen::<u8>() % 100;
        handles.push(tokio::spawn(async move {
            let connection =
                SignedConnection::new(&node_address, KeyPair::new(controller.signer().clone()))
                    .await;
            connection.validate(prc, TxStatus::InBlock).await.unwrap();
        }));
    }

    join_all(handles).await;
}

/// For a specific validator given by index, generates a predetermined number of nominator accounts.
/// Nominator accounts are produced as stashes with initial endowments.
async fn generate_nominator_accounts_with_minimal_bond<S: SignedConnectionApi>(
    connection: &S,
    validator_number: u32,
    validators_count: u32,
) -> Vec<AccountId> {
    info!(
        "Generating nominator accounts for validator {}",
        validator_number
    );
    let mut stash_accounts = vec![];
    (0..NOMINATOR_COUNT).for_each(|nominator_number| {
        let idx = validators_count + nominator_number + NOMINATOR_COUNT * validator_number;
        let stash = keypair_from_string(&format!("//{idx}//Stash"));
        stash_accounts.push(stash.account_id().clone());
    });
    for chunk in stash_accounts.chunks(TRANSFER_CALL_BATCH_LIMIT) {
        // potentially change to + 1
        connection
            .batch_transfer(chunk, MIN_NOMINATOR_BOND * 10, TxStatus::InBlock)
            .await
            .unwrap();
    }

    stash_accounts
}

async fn payout_stakers_and_assert_locked_balance(
    stash_connection: &SignedConnection,
    accounts_to_check_balance: &[AccountId],
    stash_account: &AccountId,
    era: EraIndex,
) {
    let locked_stash_balances_before_payout = stash_connection
        .locks(accounts_to_check_balance, None)
        .await;
    stash_connection
        .payout_stakers(stash_account.clone(), era - 1, TxStatus::Finalized)
        .await
        .unwrap();
    let locked_stash_balances_after_payout = stash_connection
        .locks(accounts_to_check_balance, None)
        .await;
    locked_stash_balances_before_payout.iter()
        .zip(locked_stash_balances_after_payout.iter())
        .zip(accounts_to_check_balance.iter())
        .for_each(|((balances_before, balances_after), account_id)| {
            assert!(balances_after[0].amount > balances_before[0].amount,
                    "Expected payout to be positive in locked balance for account {}. Balance before: {}, balance after: {}",
                    account_id, balances_before[0].amount, balances_after[0].amount);
        });
}
