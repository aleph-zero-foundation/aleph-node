use std::collections::{HashMap, HashSet};

use aleph_client::{
    account_from_keypair, balances_batch_transfer, balances_transfer, bond_extra_stake,
    change_validators, get_block_hash, get_committee_seats, get_current_session, get_era,
    get_era_reward_points, get_era_validators, get_exposure, get_session_first_block,
    get_session_period, get_validator_block_count, rotate_keys, set_keys, wait_for_at_least_era,
    wait_for_at_least_session, wait_for_finalized_block, wait_for_next_era, AccountId, ReadStorage,
    RewardPoint, SessionKeys, SignedConnection, XtStatus,
};
use log::{debug, info};
use pallet_elections::LENIENT_THRESHOLD;
use pallet_staking::Exposure;
use primitives::{
    Balance, BlockHash, CommitteeSeats, EraIndex, EraValidators, SessionIndex, TOKEN,
};
use sp_runtime::Perquintill;

use crate::{
    accounts::{get_validators_keys, get_validators_seeds, NodeKeys},
    Config,
};

const COMMITTEE_SEATS: CommitteeSeats = CommitteeSeats {
    reserved_seats: 2,
    non_reserved_seats: 2,
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
    validator_reward_points: HashMap<AccountId, f64>,
    retrieved_reward_points: HashMap<AccountId, u32>,
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

        debug!(
            "{} reward_ratio: {}/{}; retrieved_ratio: {}/{}",
            account, reward, our_sum, retrieved_reward, retrieved_sum
        );
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

pub fn check_points(
    connection: &SignedConnection,
    session: SessionIndex,
    era: EraIndex,
    members: impl IntoIterator<Item = AccountId>,
    members_bench: impl IntoIterator<Item = AccountId>,
    members_per_session: u32,
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

    let validator_reward_points_current_session: HashMap<AccountId, RewardPoint> =
        validator_reward_points_current_era
            .into_iter()
            .map(|(account_id, reward_points)| {
                let reward_points_previous_session = validator_reward_points_previous_session
                    .get(&account_id)
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
            account_id.clone(),
            get_node_performance(
                connection,
                &account_id,
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

pub fn get_era_for_session<C: ReadStorage>(connection: &C, session: SessionIndex) -> EraIndex {
    let session_first_block = get_session_first_block(connection, session);
    get_era(connection, Some(session_first_block))
}

pub fn setup_validators(
    config: &Config,
) -> anyhow::Result<(EraValidators<AccountId>, CommitteeSeats, SessionIndex)> {
    let root_connection = config.create_root_connection();
    // we need to wait for at least era 1 since some of the storage items are not available at era 0
    wait_for_at_least_era(&root_connection, 1)?;

    let seats = COMMITTEE_SEATS;
    let members_seats = seats.reserved_seats + seats.non_reserved_seats;
    let members_seats = members_seats.try_into().unwrap();
    let members: Vec<_> = get_validators_keys(config)
        .iter()
        .map(account_from_keypair)
        .collect();
    let members_size = members.len();

    assert!(members_size >= members_seats);

    let free_seats = members_size - members_seats;
    let reserved_free_seats = free_seats / 2;

    let reserved_size = seats.reserved_seats as usize + reserved_free_seats;
    let reserved_members = &members[0..reserved_size];
    let non_reserved_members = &members[reserved_size..];

    let session = get_current_session(&root_connection);
    let network_validators = get_era_validators(&root_connection, session);
    let first_block_in_session = get_session_first_block(&root_connection, session);
    let network_seats = get_committee_seats(&root_connection, Some(first_block_in_session));

    let era_validators = EraValidators {
        reserved: reserved_members.to_vec(),
        non_reserved: non_reserved_members.to_vec(),
    };

    if era_validators == network_validators && seats == network_seats {
        // nothing to do here
        return Ok((era_validators, seats, session));
    }

    change_validators(
        &root_connection,
        Some(reserved_members.into()),
        Some(non_reserved_members.into()),
        Some(seats),
        XtStatus::Finalized,
    );

    wait_for_next_era(&root_connection)?;
    let session = get_current_session(&root_connection);

    let network_validators = get_era_validators(&root_connection, session);
    let reserved: HashSet<_> = era_validators.reserved.iter().cloned().collect();
    let network_reserved: HashSet<_> = network_validators.reserved.into_iter().collect();
    let non_reserved: HashSet<_> = era_validators.non_reserved.iter().cloned().collect();
    let network_non_reserved: HashSet<_> = network_validators.non_reserved.into_iter().collect();
    let first_block_in_session = get_session_first_block(&root_connection, session);
    let network_seats = get_committee_seats(&root_connection, Some(first_block_in_session));

    assert_eq!(reserved, network_reserved);
    assert_eq!(non_reserved, network_non_reserved);
    assert_eq!(seats, network_seats);

    Ok((era_validators, seats, session))
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
