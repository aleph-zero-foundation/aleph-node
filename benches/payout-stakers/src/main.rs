use log::{info, trace};
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
use primitives::staking::{
    MAX_NOMINATORS_REWARDED_PER_VALIDATOR, MIN_NOMINATOR_BOND, MIN_VALIDATOR_BOND,
};
use sp_core::crypto::AccountId32;

// testcase parameters
const NOMINATOR_COUNT: u64 = MAX_NOMINATORS_REWARDED_PER_VALIDATOR as u64;
const VALIDATOR_COUNT: u64 = 10;
const ERAS_TO_WAIT: u64 = 10;

// we need to schedule batches for limited call count, otherwise we'll exhaust a block max weight
const BOND_CALL_BATCH_LIMIT: usize = 256;
const NOMINATE_CALL_BATCH_LIMIT: usize = 192;
const TRANSFER_CALL_BATCH_LIMIT: usize = 1024;

fn main() -> Result<(), anyhow::Error> {
    let address = "127.0.0.1:9944";
    let sudoer = AccountKeyring::Alice.pair();

    env_logger::init();
    info!("Make sure you have 10 nodes run in the background, otherwise you'll see extrinsic send failed errors.");

    let connection = create_connection(address).set_signer(sudoer);
    let validators = set_validators(address);
    let validators_and_its_nominators =
        create_test_validators_and_its_nominators(&connection, validators);
    wait_for_10_eras(address, &connection, validators_and_its_nominators)?;

    Ok(())
}

fn create_test_validators_and_its_nominators(
    connection: &Connection,
    validators: Vec<KeyPair>,
) -> Vec<(KeyPair, Vec<AccountId32>)> {
    validators
        .iter()
        .enumerate()
        .map(|(validator_index, validator_pair)| {
            let nominator_accounts =
                generate_nominator_accounts_with_minimal_bond(&connection, validator_index as u64);
            let nominee_account = AccountId::from(validator_pair.public());
            info!("Nominating validator {}", nominee_account);
            nominate_validator(&connection, nominator_accounts.clone(), nominee_account);
            (validator_pair.clone(), nominator_accounts)
        })
        .collect()
}

pub fn derive_user_account(seed: u64) -> KeyPair {
    trace!("Generating account from numeric seed {}", seed);
    keypair_from_string(&format!("//{}", seed))
}

fn wait_for_10_eras(
    address: &str,
    connection: &Connection,
    validators_and_its_nominators: Vec<(KeyPair, Vec<AccountId>)>,
) -> Result<(), anyhow::Error> {
    // in order to have over 4k nominators we need to wait around 60 seconds all calls to be processed
    // that means not all 4k nominators we'll make i to era 1st, hence we need to wait to 2nd era
    wait_for_next_era(connection)?;
    wait_for_next_era(connection)?;
    // then we wait another full era to test rewards
    let mut current_era = wait_for_next_era(connection)?;
    for _ in 0..ERAS_TO_WAIT {
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
            staking_batch_bond(
                connection,
                chunk,
                MIN_NOMINATOR_BOND,
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

fn set_validators(address: &str) -> Vec<KeyPair> {
    let validators = (0..VALIDATOR_COUNT)
        .map(|validator_number| derive_user_account(validator_number))
        .collect::<Vec<_>>();
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
        let connection = create_connection(address).set_signer(account.clone());
        staking_validate(&connection, 10, XtStatus::InBlock);
    });
    validators
}

fn generate_nominator_accounts_with_minimal_bond(
    connection: &Connection,
    validator_number: u64,
) -> Vec<AccountId> {
    info!(
        "Generating nominator accounts for validator {}",
        validator_number
    );
    let accounts = (0..NOMINATOR_COUNT)
        .map(|nominator_number| {
            derive_user_account(
                VALIDATOR_COUNT + nominator_number + NOMINATOR_COUNT * validator_number,
            )
        })
        .map(|key_pair| AccountId::from(key_pair.public()))
        .collect::<Vec<_>>();
    accounts
        .chunks(TRANSFER_CALL_BATCH_LIMIT)
        .for_each(|chunk| {
            balances_batch_transfer(connection, chunk.to_vec(), MIN_NOMINATOR_BOND);
        });
    accounts
}
