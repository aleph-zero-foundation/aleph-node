use frame_support::BoundedVec;
use log::info;
use pallet_staking::StakingLedger;
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
};
use sp_core::Pair;
use substrate_api_client::{AccountId, XtStatus};

use aleph_client::{
    balances_batch_transfer, change_members, get_current_session, keypair_from_string,
    payout_stakers_and_assert_locked_balance, rotate_keys, set_keys, staking_bond, staking_bonded,
    staking_ledger, staking_multi_bond, staking_nominate, staking_validate,
    wait_for_full_era_completion, wait_for_session, KeyPair, RootConnection, SignedConnection,
};
use primitives::{
    staking::{MIN_NOMINATOR_BOND, MIN_VALIDATOR_BOND},
    TOKEN,
};

use crate::{
    accounts::{accounts_from_seeds, default_account_seeds},
    config::Config,
};

fn get_key_pairs() -> (Vec<KeyPair>, Vec<KeyPair>) {
    let validators = default_account_seeds();
    let validator_stashes = validators.iter().map(|v| format!("{}//stash", v)).collect();
    let validator_accounts_key_pairs = accounts_from_seeds(&Some(validators));
    let stashes_accounts_key_pairs = accounts_from_seeds(&Some(validator_stashes));

    (stashes_accounts_key_pairs, validator_accounts_key_pairs)
}

fn convert_authorities_to_account_id(authorities: &[KeyPair]) -> Vec<AccountId> {
    authorities
        .iter()
        .map(|key| AccountId::from(key.public()))
        .collect()
}

// 1. endow stash accounts balances, controller accounts are already endowed in chainspec
// 2. bond controller account to stash account, stash = controller and set controller to StakerStatus::Validate
// 3. bond controller account to stash account, stash = controller and set controller to StakerStatus::Nominate
// 4. wait for new era
// 5. send payout stakers tx
pub fn staking_era_payouts(config: &Config) -> anyhow::Result<()> {
    let (stashes_accounts_key_pairs, validator_accounts) = get_key_pairs();

    let node = &config.node;
    let sender = validator_accounts[0].clone();
    let connection = SignedConnection::new(node, sender);
    let stashes_accounts = convert_authorities_to_account_id(&stashes_accounts_key_pairs);

    balances_batch_transfer(&connection, stashes_accounts, MIN_VALIDATOR_BOND + TOKEN);

    staking_multi_bond(node, &validator_accounts, MIN_VALIDATOR_BOND);

    validator_accounts.par_iter().for_each(|account| {
        let connection = SignedConnection::new(node, account.clone());
        staking_validate(&connection, 10, XtStatus::InBlock);
    });

    staking_multi_bond(node, &stashes_accounts_key_pairs, MIN_NOMINATOR_BOND);

    stashes_accounts_key_pairs
        .par_iter()
        .zip(validator_accounts.par_iter())
        .for_each(|(nominator, nominee)| {
            let connection = SignedConnection::new(node, nominator.clone());
            staking_nominate(&connection, nominee)
        });

    // All the above calls influence the next era, so we need to wait that it passes.
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
    let (_, mut validator_accounts) = get_key_pairs();
    let node = &config.node;
    let sender = validator_accounts.remove(0);
    // signer of this connection is sudo, the same node which in this test is used as the new one
    // it's essential since keys from rotate_keys() needs to be run against that node
    let connection: RootConnection = SignedConnection::new(node, sender).into();

    change_members(
        &connection,
        convert_authorities_to_account_id(&validator_accounts),
        XtStatus::InBlock,
    );

    let current_session = get_current_session(&connection);

    let _ = wait_for_session(&connection, current_session + 2)?;

    // to cover tx fees as we need a bit more than VALIDATOR_STAKE
    balances_batch_transfer(
        &connection.as_signed(),
        vec![stash_account.clone()],
        MIN_VALIDATOR_BOND + TOKEN,
    );
    // to cover txs fees
    balances_batch_transfer(
        &connection.as_signed(),
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
    let bonded_controller_account = staking_bonded(&connection, &stash).unwrap_or_else(|| {
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

    let validator_keys = rotate_keys(&connection).expect("Failed to retrieve keys from chain");
    let controller_connection = SignedConnection::new(node, controller.clone());
    set_keys(&controller_connection, validator_keys, XtStatus::InBlock);

    // to be elected in next era instead of expected validator_account_id
    staking_validate(&controller_connection, 10, XtStatus::InBlock);

    let ledger = staking_ledger(&connection, &controller);
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
            // we don't need to compare claimed rewards as those are internals of staking pallet
            claimed_rewards: ledger.claimed_rewards.clone()
        }
    );

    validator_accounts.push(stash);
    change_members(
        &connection,
        convert_authorities_to_account_id(&validator_accounts),
        XtStatus::InBlock,
    );
    let current_session = get_current_session(&connection);
    let _ = wait_for_session(&connection, current_session + 2)?;

    let current_era = wait_for_full_era_completion(&connection)?;
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
