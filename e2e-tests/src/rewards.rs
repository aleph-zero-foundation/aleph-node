use std::collections::HashMap;

use aleph_client::{
    account_from_keypair, balances_batch_transfer, balances_transfer, bond_extra_stake,
    get_block_hash, get_committee_seats, get_current_session, get_era_reward_points, get_exposure,
    get_session_period, get_validator_block_count, rotate_keys, set_keys,
    wait_for_at_least_session, wait_for_finalized_block, RewardPoint, SessionKeys,
    SignedConnection,
};
use log::info;
use pallet_elections::LENIENT_THRESHOLD;
use pallet_staking::Exposure;
use primitives::{Balance, BlockHash, EraIndex, SessionIndex, TOKEN};
use sp_runtime::Perquintill;
use substrate_api_client::{AccountId, XtStatus};

use crate::{
    accounts::{get_validators_seeds, NodeKeys},
    Config,
};

/// Changes session_keys used by a given `controller` to some `zero`/invalid value,
/// making it impossible to create new legal blocks.
pub fn set_invalid_keys_for_validator(
    controller_connection: &SignedConnection,
) -> anyhow::Result<()> {
    const ZERO_SESSION_KEYS: SessionKeys = SessionKeys {
        aura: [0; 32],
        aleph: [0; 32],
    };

    set_keys(controller_connection, ZERO_SESSION_KEYS, XtStatus::InBlock);
    // wait until our node is forced to use new keys, i.e. current session + 2
    let current_session = get_current_session(controller_connection);
    wait_for_at_least_session(controller_connection, current_session + 2)?;

    Ok(())
}

/// Rotates session_keys of a given `controller`, making it able to rejoin the `consensus`.
pub fn reset_validator_keys(controller_connection: &SignedConnection) -> anyhow::Result<()> {
    let validator_keys =
        rotate_keys(controller_connection).expect("Failed to retrieve keys from chain");
    set_keys(controller_connection, validator_keys, XtStatus::InBlock);

    // wait until our node is forced to use new keys, i.e. current session + 2
    let current_session = get_current_session(controller_connection);
    wait_for_at_least_session(controller_connection, current_session + 2)?;

    Ok(())
}

pub fn download_exposure(
    connection: &SignedConnection,
    era: EraIndex,
    account_id: &AccountId,
    beginning_of_session_block_hash: BlockHash,
) -> Balance {
    let exposure: Exposure<AccountId, Balance> = get_exposure(
        connection,
        era,
        account_id,
        Some(beginning_of_session_block_hash),
    );
    info!(
        "Validator {} has own exposure of {} and total of {}.",
        account_id, exposure.own, exposure.total
    );
    exposure.others.iter().for_each(|individual_exposure| {
        info!(
            "Validator {} has nominator {} exposure {}.",
            account_id, individual_exposure.who, individual_exposure.value
        )
    });
    exposure.total
}

fn check_rewards(
    validator_reward_points: HashMap<&AccountId, f64>,
    retrieved_reward_points: HashMap<&AccountId, u32>,
    max_relative_difference: f64,
) -> anyhow::Result<()> {
    let our_sum: f64 = validator_reward_points
        .iter()
        .map(|(_, reward)| reward)
        .sum();
    let retrieved_sum: u32 = retrieved_reward_points
        .iter()
        .map(|(_, reward)| reward)
        .sum();

    for (account, reward) in validator_reward_points {
        let retrieved_reward = *retrieved_reward_points.get(&account).unwrap_or_else(|| {
            panic!(
                "missing account={} in retrieved collection of reward points",
                account
            )
        });

        let reward_ratio = reward / our_sum;
        let retrieved_ratio = retrieved_reward as f64 / retrieved_sum as f64;

        info!(
            "{} reward_ratio: {}; retrieved_ratio: {}.",
            account, reward_ratio, retrieved_ratio
        );
        assert!((reward_ratio - retrieved_ratio).abs() <= max_relative_difference);
    }

    Ok(())
}

fn get_node_performance(
    connection: &SignedConnection,
    account_id: &AccountId,
    before_end_of_session_block_hash: BlockHash,
    blocks_to_produce_per_session: u32,
) -> f64 {
    let block_count = get_validator_block_count(
        connection,
        account_id,
        Some(before_end_of_session_block_hash),
    )
    .unwrap_or(0);
    info!(
        "Block count for validator {} is {:?}, block hash is {}.",
        account_id, block_count, before_end_of_session_block_hash
    );
    let performance = block_count as f64 / blocks_to_produce_per_session as f64;
    info!("validator {}, performance {:?}.", account_id, performance);
    let lenient_performance = match Perquintill::from_float(performance) >= LENIENT_THRESHOLD
        && blocks_to_produce_per_session >= block_count
    {
        true => 1.0,
        false => performance,
    };
    info!(
        "Validator {}, lenient performance {:?}.",
        account_id, lenient_performance
    );
    lenient_performance
}

pub fn check_points<'a>(
    connection: &SignedConnection,
    session: SessionIndex,
    era: EraIndex,
    members: impl IntoIterator<Item = &'a AccountId> + Clone,
    members_bench: impl IntoIterator<Item = &'a AccountId> + Clone,
    max_relative_difference: f64,
) -> anyhow::Result<()> {
    let session_period = get_session_period(connection);

    info!("Era: {} | session: {}.", era, session);

    let beggining_of_session_block = session * session_period;
    let end_of_session_block = beggining_of_session_block + session_period;
    info!("Waiting for block: {}.", end_of_session_block);
    wait_for_finalized_block(connection, end_of_session_block)?;

    let beggining_of_session_block_hash = get_block_hash(connection, beggining_of_session_block);
    let end_of_session_block_hash = get_block_hash(connection, end_of_session_block);
    let before_end_of_session_block_hash = get_block_hash(connection, end_of_session_block - 1);
    info!("End-of-session block hash: {}.", end_of_session_block_hash);

    let committee_seats = get_committee_seats(connection, Some(beggining_of_session_block_hash));
    let members_per_session = committee_seats.non_reserved_seats + committee_seats.reserved_seats;

    info!("Members per session: {}.", members_per_session);

    let blocks_to_produce_per_session = session_period / members_per_session;
    info!(
        "Blocks to produce per session: {} - session period {}.",
        blocks_to_produce_per_session, session_period
    );

    // get points stored by the Staking pallet
    let validator_reward_points_current_era =
        get_era_reward_points(connection, era, Some(end_of_session_block_hash))
            .unwrap_or_default()
            .individual;

    let validator_reward_points_previous_session =
        get_era_reward_points(connection, era, Some(beggining_of_session_block_hash))
            .unwrap_or_default()
            .individual;

    let validator_reward_points_current_session: HashMap<&AccountId, RewardPoint> =
        validator_reward_points_current_era
            .iter()
            .map(|(account_id, reward_points)| {
                let reward_points_previous_session = validator_reward_points_previous_session
                    .get(account_id)
                    .unwrap_or(&0);
                let reward_points_current = reward_points - reward_points_previous_session;

                info!(
                    "In session {} validator {} accumulated {}.",
                    session, account_id, reward_points
                );
                (account_id, reward_points_current)
            })
            .collect();

    let members_uptime = members.into_iter().map(|account_id| {
        (
            account_id,
            get_node_performance(
                connection,
                account_id,
                before_end_of_session_block_hash,
                blocks_to_produce_per_session,
            ),
        )
    });

    let members_bench_uptime = members_bench
        .into_iter()
        .map(|account_id| (account_id, 1.0));

    let mut reward_points: HashMap<_, _> = members_uptime.chain(members_bench_uptime).collect();
    let members_count = reward_points.len() as f64;
    for (account_id, reward_points) in reward_points.iter_mut() {
        let exposure =
            download_exposure(connection, era, account_id, beggining_of_session_block_hash);
        *reward_points *= exposure as f64 / members_count;
    }

    check_rewards(
        reward_points,
        validator_reward_points_current_session,
        max_relative_difference,
    )
}
pub fn get_bench_members<'a>(
    non_reserved_members: &'a [AccountId],
    non_reserved_members_for_session: &[AccountId],
) -> Vec<&'a AccountId> {
    non_reserved_members
        .iter()
        .filter(|account_id| !non_reserved_members_for_session.contains(account_id))
        .collect::<Vec<_>>()
}

pub fn validators_bond_extra_stakes(config: &Config, additional_stakes: &[Balance]) {
    let node = &config.node;
    let root_connection = config.create_root_connection();

    let accounts_keys: Vec<NodeKeys> = get_validators_seeds(config)
        .into_iter()
        .map(|seed| seed.into())
        .collect();

    let controller_accounts: Vec<AccountId> = accounts_keys
        .iter()
        .map(|account_keys| account_from_keypair(&account_keys.controller))
        .collect();

    // funds to cover fees
    balances_batch_transfer(&root_connection.as_signed(), controller_accounts, TOKEN);

    accounts_keys.iter().zip(additional_stakes.iter()).for_each(
        |(account_keys, additional_stake)| {
            let validator_id = account_from_keypair(&account_keys.validator);

            // Additional TOKEN to cover fees
            balances_transfer(
                &root_connection.as_signed(),
                &validator_id,
                *additional_stake + TOKEN,
                XtStatus::Finalized,
            );
            let stash_connection = SignedConnection::new(node, account_keys.validator.clone());
            bond_extra_stake(&stash_connection, *additional_stake);
        },
    );
}
