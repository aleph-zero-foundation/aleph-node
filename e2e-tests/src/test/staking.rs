use crate::{
    accounts::{accounts_from_seeds, default_account_seeds, keypair_from_string},
    config::Config,
    staking::{
        bond, bonded, check_non_zero_payouts_for_era, ledger, nominate, validate,
        wait_for_full_era_completion,
    },
    transfer::batch_endow_account_balances,
};
use aleph_client::{
    change_members, create_connection, get_current_session, rotate_keys, set_keys,
    wait_for_session, KeyPair,
};
use log::info;
use pallet_staking::StakingLedger;
use primitives::{
    staking::{MIN_NOMINATOR_BOND, MIN_VALIDATOR_BOND},
    TOKEN,
};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
};
use sp_core::Pair;
use substrate_api_client::{AccountId, XtStatus};

fn get_key_pairs() -> (Vec<KeyPair>, Vec<KeyPair>) {
    let validators = default_account_seeds();
    let validator_stashes: Vec<_> = validators
        .iter()
        .map(|v| String::from(v) + "//stash")
        .collect();
    let validator_accounts_key_pairs = accounts_from_seeds(&Some(validators));
    let stashes_accounts_key_pairs = accounts_from_seeds(&Some(validator_stashes));

    (stashes_accounts_key_pairs, validator_accounts_key_pairs)
}

fn convert_authorities_to_account_id(authorities: Vec<KeyPair>) -> Vec<AccountId> {
    authorities
        .into_iter()
        .map(|key| AccountId::from(key.public()))
        .collect()
}

// 1. endow stash accounts balances, controller accounts are already endowed in chainspec
// 2. bond controller account to stash account, stash = controller and set controller to StakerStatus::Validate
// 3. bond controller account to stash account, stash = controller and set controller to StakerStatus::Nominate
// 4. wait for new era
// 5. send payout stakers tx
pub fn staking_era_payouts(config: &Config) -> anyhow::Result<()> {
    let (stashes_accounts, validator_accounts) = get_key_pairs();

    let node = &config.node;
    let sender = validator_accounts[0].clone();
    let connection = create_connection(node).set_signer(sender);

    batch_endow_account_balances(&connection, &stashes_accounts, MIN_VALIDATOR_BOND + TOKEN);

    validator_accounts.par_iter().for_each(|account| {
        bond(node, MIN_VALIDATOR_BOND, &account, &account);
    });

    validator_accounts
        .par_iter()
        .for_each(|account| validate(node, account, XtStatus::InBlock));

    stashes_accounts
        .par_iter()
        .for_each(|nominator| bond(node, MIN_NOMINATOR_BOND, &nominator, &nominator));

    stashes_accounts
        .par_iter()
        .zip(validator_accounts.par_iter())
        .for_each(|(nominator, nominee)| nominate(node, nominator, nominee));

    // All the above calls influace the next era, so we need to wait that it passes.
    let current_era = wait_for_full_era_completion(&connection)?;
    info!(
        "Era {} started, claiming rewards for era {}",
        current_era,
        current_era - 1
    );

    validator_accounts.into_par_iter().for_each(|key_pair| {
        check_non_zero_payouts_for_era(node, &key_pair, &connection, current_era)
    });

    Ok(())
}

// 1. decrease number of validators from 4 to 3
// 2. endow stash account balances
// 3. bond controller account to the stash account, stash != controller and set controller to StakerStatus::Validate
// 4. call bonded, double check bonding
// 5. set keys for controller account from validator's rotate_keys()
// 6. set controller to StakerStatus::Validate, call ledger to double check storage state
// 7. add 4th validator which is the new stash account
// 8. wait for next era
// 9. claim rewards for the stash account
pub fn staking_new_validator(config: &Config) -> anyhow::Result<()> {
    let controller = keypair_from_string("//Controller");
    let controller_account = AccountId::from(controller.public());
    let stash_seed = "//Stash";
    let stash = keypair_from_string(stash_seed);
    let stash_account = AccountId::from(stash.public());
    let (_, mut validator_accounts) = get_key_pairs();
    let node = &config.node;
    let sender = validator_accounts.remove(0);
    // signer of this connection is sudo, the same node which in this test is used as the new one
    // it's essential since keys from rotate_keys() needs to be run against that node
    let connection = create_connection(node).set_signer(sender);

    change_members(
        &connection,
        convert_authorities_to_account_id(validator_accounts.clone()),
        XtStatus::InBlock,
    );

    let current_session = get_current_session(&connection);
    let _ = wait_for_session(&connection, current_session + 2)?;

    // to cover tx fees as we need a bit more than VALIDATOR_STAKE
    batch_endow_account_balances(&connection, &[stash.clone()], MIN_VALIDATOR_BOND + TOKEN);
    // to cover txs fees
    batch_endow_account_balances(&connection, &[controller.clone()], TOKEN);

    bond(node, MIN_VALIDATOR_BOND, &stash, &controller);
    let bonded_controller_account = bonded(&connection, &stash);
    assert!(
        bonded_controller_account.is_some(),
        "Expected that stash account {} is bonded to some controller!",
        &stash_account
    );
    let bonded_controller_account = bonded_controller_account.unwrap();
    assert_eq!(
        bonded_controller_account, controller_account,
        "Expected that stash account {} is bonded to the controller account {}, got {} instead!",
        &stash_account, &controller_account, &bonded_controller_account
    );

    let validator_keys = rotate_keys(&connection).unwrap().unwrap();
    let controller_connection = create_connection(node).set_signer(controller.clone());
    set_keys(&controller_connection, validator_keys, XtStatus::InBlock);

    // to be elected in next era instead of expected validator_account_id
    validate(node, &controller, XtStatus::InBlock);

    let ledger = ledger(&connection, &controller);
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
            unlocking: vec![],
            // we don't need to compare claimed rewards as those are internals of staking pallet
            claimed_rewards: ledger.claimed_rewards.clone()
        }
    );

    validator_accounts.push(stash.clone());
    change_members(
        &connection,
        convert_authorities_to_account_id(validator_accounts.clone()),
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

    check_non_zero_payouts_for_era(node, &stash, &connection, current_era);

    Ok(())
}
