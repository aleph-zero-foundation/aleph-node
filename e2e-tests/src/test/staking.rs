use aleph_client::{
    account_from_keypair,
    api::runtime_types::sp_core::bounded::bounded_vec::BoundedVec,
    keypair_from_string,
    pallet_staking::StakingLedger,
    pallets::{
        author::AuthorRpc,
        balances::{BalanceApi, BalanceUserApi, BalanceUserBatchExtApi},
        elections::ElectionsSudoApi,
        session::SessionUserApi,
        staking::{StakingApi, StakingUserApi},
    },
    primitives::CommitteeSeats,
    waiting::{BlockStatus, WaitingExt},
    AccountId, KeyPair, Pair, SignedConnection, SignedConnectionApi, TxStatus,
};
use log::info;
use primitives::{
    staking::{MIN_NOMINATOR_BOND, MIN_VALIDATOR_BOND},
    Balance, BlockNumber, TOKEN,
};

use crate::{
    accounts::{account_ids_from_keys, accounts_seeds_to_keys, get_validators_seeds},
    config::{setup_test, Config},
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
#[tokio::test]
pub async fn staking_era_payouts() -> anyhow::Result<()> {
    let config = setup_test();
    let (stashes_accounts_key_pairs, validator_accounts) = get_validator_stashes_key_pairs(config);

    let node = &config.node;
    let connection = config.get_first_signed_connection().await;
    let stashes_accounts = account_ids_from_keys(&stashes_accounts_key_pairs);

    connection
        .batch_transfer(
            &stashes_accounts,
            MIN_NOMINATOR_BOND + TOKEN,
            TxStatus::InBlock,
        )
        .await?;
    multi_bond(node, &stashes_accounts_key_pairs, MIN_NOMINATOR_BOND).await;

    for (nominator, nominee) in stashes_accounts_key_pairs
        .into_iter()
        .zip(validator_accounts)
    {
        let connection = SignedConnection::new(node, nominator).await;
        let nominee_account_id = AccountId::from(nominee.signer().public());
        connection
            .nominate(nominee_account_id, TxStatus::InBlock)
            .await?;
    }

    // All the above calls influence the next era, so we need to wait that it passes.
    // this test can be speeded up by forcing new era twice, and waiting 4 sessions in total instead of almost 10 sessions
    connection.wait_for_n_eras(2, BlockStatus::Finalized).await;
    let current_era = connection.get_current_era(None).await;
    info!(
        "Era {} started, claiming rewards for era {}",
        current_era,
        current_era - 1
    );

    let (_, validator_accounts) = get_validator_stashes_key_pairs(config);
    for key_pair in validator_accounts {
        let stash_account = AccountId::from(key_pair.signer().public());
        let stash_connection = SignedConnection::new(node, key_pair).await;
        payout_stakers_and_assert_locked_balance(
            &stash_connection,
            &[stash_account.clone()],
            &stash_account,
            current_era,
        )
        .await;
    }

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
#[tokio::test]
pub async fn staking_new_validator() -> anyhow::Result<()> {
    let config = setup_test();
    let controller_seed = "//Controller";
    let controller = keypair_from_string(controller_seed);
    let controller_account = AccountId::from(controller.signer().public());
    let stash_seed = "//Stash";
    let stash = keypair_from_string(stash_seed);
    let stash_account = AccountId::from(stash.signer().public());
    let (_, mut validator_accounts) = get_validator_stashes_key_pairs(config);
    let node = &config.node;
    let _ = validator_accounts.remove(0);
    // signer of this connection is sudo, the same node which in this test is used as the new one
    // it's essential since keys from rotate_keys() needs to be run against that node
    let root_connection = config.create_root_connection().await;

    root_connection
        .change_validators(
            Some(account_ids_from_keys(&validator_accounts)),
            Some(vec![]),
            Some(CommitteeSeats {
                reserved_seats: 4,
                non_reserved_seats: 0,
                non_reserved_finality_seats: 0,
            }),
            TxStatus::InBlock,
        )
        .await?;

    root_connection
        .wait_for_n_sessions(2, BlockStatus::Best)
        .await;

    // to cover tx fees as we need a bit more than VALIDATOR_STAKE
    root_connection
        .transfer(
            stash_account.clone(),
            MIN_VALIDATOR_BOND + TOKEN,
            TxStatus::InBlock,
        )
        .await?;
    // to cover txs fees
    root_connection
        .transfer(controller_account.clone(), TOKEN, TxStatus::InBlock)
        .await?;

    let stash_connection = SignedConnection::new(node, KeyPair::new(stash.signer().clone())).await;

    stash_connection
        .bond(
            MIN_VALIDATOR_BOND,
            controller_account.clone(),
            TxStatus::InBlock,
        )
        .await?;

    let bonded_controller_account = root_connection
        .get_bonded(stash_account.clone(), None)
        .await
        .expect("should be bonded to smth");
    assert_eq!(
        bonded_controller_account, controller_account,
        "Expected that stash account {} is bonded to the controller account {}, got {} instead!",
        &stash_account, &controller_account, &bonded_controller_account
    );

    let validator_keys = root_connection.author_rotate_keys().await?;
    let controller_connection =
        SignedConnection::new(node, KeyPair::new(controller.signer().clone())).await;
    controller_connection
        .set_keys(validator_keys, TxStatus::InBlock)
        .await?;
    controller_connection
        .validate(10, TxStatus::InBlock)
        .await?;
    let ledger = controller_connection
        .get_ledger(controller_account, None)
        .await;
    assert_eq!(
        ledger,
        StakingLedger {
            stash: stash_account.clone(),
            total: MIN_VALIDATOR_BOND,
            active: MIN_VALIDATOR_BOND,
            unlocking: BoundedVec(vec![]),
            // since era is 3 sessions, validate is done in the first block of 2nd session,
            // that is already after elections has been done for 1st era
            claimed_rewards: BoundedVec(vec![0]),
        }
    );

    validator_accounts.push(stash);
    root_connection
        .change_validators(
            Some(account_ids_from_keys(&validator_accounts)),
            Some(vec![]),
            Some(CommitteeSeats {
                reserved_seats: 5,
                non_reserved_seats: 0,
                non_reserved_finality_seats: 0,
            }),
            TxStatus::InBlock,
        )
        .await?;
    root_connection
        .wait_for_n_sessions(2, BlockStatus::Best)
        .await;
    root_connection.wait_for_n_eras(2, BlockStatus::Best).await;
    let current_era = root_connection.get_current_era(None).await;
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
    )
    .await;

    Ok(())
}

pub async fn multi_bond(node: &str, bonders: &[KeyPair], stake: Balance) {
    for bonder in bonders {
        let controller_account = account_from_keypair(bonder.signer());
        let connection = SignedConnection::new(node, KeyPair::new(bonder.signer().clone())).await;
        connection
            .bond(stake, controller_account, TxStatus::InBlock)
            .await
            .unwrap();
    }
}

async fn payout_stakers_and_assert_locked_balance<S: SignedConnectionApi>(
    stash_connection: &S,
    accounts_to_check_balance: &[AccountId],
    stash_account: &AccountId,
    era: BlockNumber,
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
