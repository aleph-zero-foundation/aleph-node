use aleph_client::{
    balances_batch_transfer, change_validators, get_current_session, keypair_from_string,
    payout_stakers_and_assert_locked_balance, rotate_keys, set_keys, staking_bond, staking_bonded,
    staking_ledger, staking_multi_bond, staking_nominate, staking_validate,
    wait_for_full_era_completion, wait_for_session, AccountId, KeyPair, SignedConnection,
    StakingLedger, XtStatus,
};
use frame_support::BoundedVec;
use log::info;
use primitives::{
    staking::{MIN_NOMINATOR_BOND, MIN_VALIDATOR_BOND},
    CommitteeSeats, TOKEN,
};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
};
use sp_core::Pair;

use crate::{
    accounts::{account_ids_from_keys, accounts_seeds_to_keys, get_validators_seeds},
    config::Config,
};

fn get_validator_stashes_key_pairs(config: &Config) -> (Vec<KeyPair>, Vec<KeyPair>) {
    let validators_seeds = get_validators_seeds(config);
    let validator_stashes: Vec<_> = validators_seeds
        .iter()
        .map(|v| format!("{}//stash", v))
        .collect();
    let validator_accounts_key_pairs = accounts_seeds_to_keys(&validators_seeds);
    let stashes_accounts_key_pairs = accounts_seeds_to_keys(&validator_stashes);

    (stashes_accounts_key_pairs, validator_accounts_key_pairs)
}

// 0. validators stash and controllers are already endowed, bonded and validated in a genesis block
// 1. endow nominators stash accounts balances
// 3. bond controller account to stash account, stash = controller and set controller to StakerStatus::Nominate
// 4. wait for new era
// 5. send payout stakers tx
pub fn staking_era_payouts(config: &Config) -> anyhow::Result<()> {
    let (stashes_accounts_key_pairs, validator_accounts) = get_validator_stashes_key_pairs(config);

    let node = &config.node;
    let connection = config.get_first_signed_connection();
    let stashes_accounts = account_ids_from_keys(&stashes_accounts_key_pairs);

    balances_batch_transfer(&connection, stashes_accounts, MIN_NOMINATOR_BOND + TOKEN);
    staking_multi_bond(node, &stashes_accounts_key_pairs, MIN_NOMINATOR_BOND);

    stashes_accounts_key_pairs
        .par_iter()
        .zip(validator_accounts.par_iter())
        .for_each(|(nominator, nominee)| {
            let connection = SignedConnection::new(node, nominator.clone());
            let nominee_account_id = AccountId::from(nominee.public());
            staking_nominate(&connection, &nominee_account_id)
        });

    // All the above calls influence the next era, so we need to wait that it passes.
    // this test can be speeded up by forcing new era twice, and waiting 4 sessions in total instead of almost 10 sessions
    let current_era = wait_for_full_era_completion(&connection)?;
    info!(
        "Era {} started, claiming rewards for era {}",
        current_era,
        current_era - 1
    );

    validator_accounts.into_par_iter().for_each(|key_pair| {
        let stash_connection = SignedConnection::new(node, key_pair.clone());
        let stash_account = AccountId::from(key_pair.public());
        payout_stakers_and_assert_locked_balance(
            &stash_connection,
            &[stash_account.clone()],
            &stash_account,
            current_era,
        )
    });

    Ok(())
}

// 1. decrease number of validators from 4 to 3
// 2. endow stash account balances
// 3. bond controller account to the stash account, stash != controller and set controller to StakerStatus::Validate
// 4. call bonded, double check bonding
// 5. set keys for controller account from validator's rotate_keys()
// 6. set controller to StakerStatus::Validate, call ledger to double-check storage state
// 7. add 4th validator which is the new stash account
// 8. wait for next era
// 9. claim rewards for the stash account
pub fn staking_new_validator(config: &Config) -> anyhow::Result<()> {
    let controller_seed = "//Controller";
    let controller = keypair_from_string(controller_seed);
    let controller_account = AccountId::from(controller.public());
    let stash_seed = "//Stash";
    let stash = keypair_from_string(stash_seed);
    let stash_account = AccountId::from(stash.public());
    let (_, mut validator_accounts) = get_validator_stashes_key_pairs(config);
    let node = &config.node;
    let _ = validator_accounts.remove(0);
    // signer of this connection is sudo, the same node which in this test is used as the new one
    // it's essential since keys from rotate_keys() needs to be run against that node
    let root_connection = config.create_root_connection();

    change_validators(
        &root_connection,
        Some(account_ids_from_keys(&validator_accounts)),
        Some(vec![]),
        Some(CommitteeSeats {
            reserved_seats: 4,
            non_reserved_seats: 0,
        }),
        XtStatus::InBlock,
    );

    let current_session = get_current_session(&root_connection);

    let _ = wait_for_session(&root_connection, current_session + 2)?;

    // to cover tx fees as we need a bit more than VALIDATOR_STAKE
    balances_batch_transfer(
        &root_connection.as_signed(),
        vec![stash_account.clone()],
        MIN_VALIDATOR_BOND + TOKEN,
    );
    // to cover txs fees
    balances_batch_transfer(
        &root_connection.as_signed(),
        vec![controller_account.clone()],
        TOKEN,
    );

    let stash_connection = SignedConnection::new(node, stash.clone());

    staking_bond(
        &stash_connection,
        MIN_VALIDATOR_BOND,
        &controller_account,
        XtStatus::InBlock,
    );
    let bonded_controller_account = staking_bonded(&root_connection, &stash).unwrap_or_else(|| {
        panic!(
            "Expected that stash account {} is bonded to some controller!",
            &stash_account
        )
    });
    assert_eq!(
        bonded_controller_account, controller_account,
        "Expected that stash account {} is bonded to the controller account {}, got {} instead!",
        &stash_account, &controller_account, &bonded_controller_account
    );

    let validator_keys = rotate_keys(&root_connection).expect("Failed to retrieve keys from chain");
    let controller_connection = SignedConnection::new(node, controller.clone());
    set_keys(&controller_connection, validator_keys, XtStatus::InBlock);

    // to be elected in next era instead of expected validator_account_id
    staking_validate(&controller_connection, 10, XtStatus::InBlock);

    let ledger = staking_ledger(&root_connection, &controller);
    assert!(
        ledger.is_some(),
        "Expected controller {} configuration to be non empty",
        controller_account
    );
    let ledger = ledger.unwrap();
    assert_eq!(
        ledger,
        StakingLedger {
            stash: stash_account.clone(),
            total: MIN_VALIDATOR_BOND,
            active: MIN_VALIDATOR_BOND,
            unlocking: BoundedVec::try_from(vec![]).unwrap(),
        }
    );

    validator_accounts.push(stash);
    change_validators(
        &root_connection,
        Some(account_ids_from_keys(&validator_accounts)),
        Some(vec![]),
        Some(CommitteeSeats {
            reserved_seats: 5,
            non_reserved_seats: 0,
        }),
        XtStatus::InBlock,
    );
    let current_session = get_current_session(&root_connection);
    let _ = wait_for_session(&root_connection, current_session + 2)?;

    let current_era = wait_for_full_era_completion(&root_connection)?;
    info!(
        "Era {} started, claiming rewards for era {}",
        current_era,
        current_era - 1
    );

    payout_stakers_and_assert_locked_balance(
        &stash_connection,
        &[stash_account.clone()],
        &stash_account,
        current_era,
    );

    Ok(())
}
