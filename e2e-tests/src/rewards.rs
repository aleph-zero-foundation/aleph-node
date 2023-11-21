use std::collections::{HashMap, HashSet};

use aleph_client::{
    account_from_keypair,
    pallets::{
        author::AuthorRpc,
        balances::BalanceUserApi,
        committee_management::CommitteeManagementApi,
        elections::{ElectionsApi, ElectionsSudoApi},
        session::{SessionApi, SessionUserApi},
        staking::{StakingApi, StakingUserApi},
    },
    primitives::{CommitteeSeats, EraValidators},
    utility::BlocksApi,
    waiting::{AlephWaiting, BlockStatus, WaitingExt},
    AccountId, AsConnection, SignedConnection, TxStatus,
};
use anyhow::anyhow;
use log::{debug, info};
use primitives::{Balance, BlockHash, EraIndex, SessionIndex, LENIENT_THRESHOLD, TOKEN};
use rand::Rng;
use sp_runtime::Perquintill;

use crate::{
    accounts::{get_validators_keys, get_validators_seeds, NodeKeys},
    config::Config,
};

const COMMITTEE_SEATS: CommitteeSeats = CommitteeSeats {
    reserved_seats: 2,
    non_reserved_seats: 2,
    non_reserved_finality_seats: 2,
};
type RewardPoint = u32;

/// Changes session_keys used by a given `controller` to some `zero`/invalid value,
/// making it impossible to create new legal blocks.
pub async fn set_invalid_keys_for_validator<S: WaitingExt + SessionUserApi>(
    controller_connections: Vec<S>,
) -> anyhow::Result<()> {
    if controller_connections.is_empty() {
        return Ok(());
    }

    let mut rng = rand::thread_rng();
    for con in &controller_connections {
        let mut invalid_keys = [0u8; 64];
        rng.fill(&mut invalid_keys);

        con.set_keys(invalid_keys.to_vec().into(), TxStatus::Finalized)
            .await
            .unwrap();
    }
    // wait until our node is forced to use new keys, i.e. current session + 2
    controller_connections[0]
        .wait_for_n_sessions(2, BlockStatus::Best)
        .await;

    Ok(())
}

/// Rotates session_keys of a given `controller`, making it able to rejoin the `consensus`.
pub(super) async fn reset_validator_keys<S: AuthorRpc + WaitingExt + SessionUserApi>(
    controller_connection: &S,
) -> anyhow::Result<()> {
    let validator_keys = controller_connection.author_rotate_keys().await?;
    controller_connection
        .set_keys(validator_keys, TxStatus::InBlock)
        .await
        .unwrap();

    // wait until our node is forced to use new keys, i.e. current session + 2
    controller_connection
        .wait_for_n_sessions(2, BlockStatus::Best)
        .await;

    Ok(())
}

pub async fn download_exposure<S: StakingApi>(
    connection: &S,
    era: EraIndex,
    account_id: &AccountId,
    beginning_of_session_block_hash: BlockHash,
) -> Balance {
    let exposure = connection
        .get_exposure(era, account_id, Some(beginning_of_session_block_hash))
        .await;
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
    let our_sum: f64 = validator_reward_points.values().sum();
    let retrieved_sum: u32 = retrieved_reward_points.values().sum();

    for (account, reward) in &validator_reward_points {
        let reward = *reward;
        let retrieved_reward = *retrieved_reward_points.get(account).unwrap_or_else(|| {
            panic!(
                "missing account={account} in retrieved collection of reward points {validator_reward_points:?}"
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

async fn get_node_performance<S: ElectionsApi + CommitteeManagementApi>(
    connection: &S,
    account_id: &AccountId,
    before_end_of_session_block_hash: BlockHash,
    blocks_to_produce_per_session: u32,
) -> f64 {
    let block_count = connection
        .get_validator_block_count(account_id.clone(), Some(before_end_of_session_block_hash))
        .await
        .unwrap_or_default();

    info!(
        "Block count for validator {} is {:?}, block hash is {}.",
        account_id, block_count, before_end_of_session_block_hash
    );
    let performance = block_count as f64 / blocks_to_produce_per_session as f64;
    info!("validator {}, performance {:?}.", account_id, performance);
    let lenient_performance = match Perquintill::from_float(performance) >= LENIENT_THRESHOLD {
        true => 1.0,
        false => performance,
    };
    info!(
        "Validator {}, lenient performance {:?}.",
        account_id, lenient_performance
    );
    lenient_performance
}

pub async fn check_points<S: AsConnection + Sync>(
    connection: &S,
    session: SessionIndex,
    era: EraIndex,
    members: impl IntoIterator<Item = AccountId>,
    members_bench: impl IntoIterator<Item = AccountId>,
    members_per_session: u32,
    max_relative_difference: f64,
) -> anyhow::Result<()> {
    let session_period = connection.get_session_period().await?;

    info!("Era: {} | session: {}.", era, session);

    let beginning_of_session_block = session * session_period;
    let end_of_session_block = beginning_of_session_block + session_period;
    info!("Waiting for block: {}.", end_of_session_block);
    connection
        .wait_for_block(|n| n >= end_of_session_block, BlockStatus::Finalized)
        .await;

    let beginning_of_session_block_hash = connection
        .get_block_hash(beginning_of_session_block)
        .await?;
    let end_of_session_block_hash = connection.get_block_hash(end_of_session_block).await?;
    let before_end_of_session_block_hash =
        connection.get_block_hash(end_of_session_block - 1).await?;
    info!(
        "End-of-session block hash: {:?}.",
        end_of_session_block_hash
    );

    info!("Members per session: {}.", members_per_session);

    let blocks_to_produce_per_session = session_period / members_per_session;
    info!(
        "Blocks to produce per session: {} - session period {}.",
        blocks_to_produce_per_session, session_period
    );

    // get points stored by the Staking pallet
    let validator_reward_points_current_era = connection
        .get_era_reward_points(era, end_of_session_block_hash)
        .await
        .unwrap_or_default()
        .individual;

    let validator_reward_points_previous_session = HashMap::<AccountId, u32>::from_iter(
        connection
            .get_era_reward_points(era, beginning_of_session_block_hash)
            .await
            .unwrap_or_default()
            .individual,
    );

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

    let mut members_uptime = vec![];
    for account_id in members.into_iter() {
        members_uptime.push((
            account_id.clone(),
            get_node_performance(
                connection,
                &account_id,
                before_end_of_session_block_hash.unwrap(),
                blocks_to_produce_per_session,
            )
            .await,
        ));
    }

    let members_bench_uptime = members_bench
        .into_iter()
        .map(|account_id| (account_id, 1.0));

    let mut reward_points: HashMap<_, _> = members_uptime
        .into_iter()
        .chain(members_bench_uptime)
        .collect();

    let members_count = reward_points.len() as f64;
    for (account_id, reward_points) in reward_points.iter_mut() {
        let exposure = download_exposure(
            connection,
            era,
            account_id,
            beginning_of_session_block_hash.unwrap(),
        )
        .await;
        *reward_points *= exposure as f64 / members_count;
    }

    check_rewards(
        reward_points,
        validator_reward_points_current_session,
        max_relative_difference,
    )
}

pub async fn setup_validators(
    config: &Config,
) -> anyhow::Result<(EraValidators<AccountId>, CommitteeSeats, SessionIndex)> {
    let root_connection = config.create_root_connection().await;
    // we need to wait for at least era 1 since some of the storage items are not available at era 0
    root_connection.wait_for_n_eras(1, BlockStatus::Best).await;

    let seats = COMMITTEE_SEATS;
    let members_seats = seats.reserved_seats + seats.non_reserved_seats;
    let members_seats = members_seats.try_into().unwrap();
    let members: Vec<_> = get_validators_keys(config)
        .iter()
        .map(|kp| account_from_keypair(kp.signer()))
        .collect();
    let members_size = members.len();

    assert!(members_size >= members_seats);

    let free_seats = members_size - members_seats;
    let reserved_free_seats = free_seats / 2;

    let reserved_size = seats.reserved_seats as usize + reserved_free_seats;
    let reserved_members = &members[0..reserved_size];
    let non_reserved_members = &members[reserved_size..];

    let session = root_connection.get_session(None).await;
    let network_validators = root_connection.get_current_era_validators(None).await;
    let first_block_in_session = root_connection
        .first_block_of_session(session)
        .await?
        .ok_or(anyhow!("First block of session {} is None!", session))?;
    let network_seats = root_connection
        .get_committee_seats(Some(first_block_in_session))
        .await;

    let era_validators = EraValidators {
        reserved: reserved_members.to_vec(),
        non_reserved: non_reserved_members.to_vec(),
    };

    if era_validators == network_validators && seats == network_seats {
        // nothing to do here
        return Ok((era_validators, seats, session));
    }

    info!("changing validators to {:?}", era_validators);
    root_connection
        .change_validators(
            Some(reserved_members.into()),
            Some(non_reserved_members.into()),
            Some(seats.clone()),
            TxStatus::Finalized,
        )
        .await?;

    root_connection.wait_for_n_eras(2, BlockStatus::Best).await;
    let session = root_connection.get_session(None).await;

    let first_block_in_session = root_connection.first_block_of_session(session).await?;
    let network_validators = root_connection
        .get_current_era_validators(first_block_in_session)
        .await;
    let reserved: HashSet<_> = era_validators.reserved.iter().cloned().collect();
    let network_reserved: HashSet<_> = network_validators.reserved.into_iter().collect();
    let non_reserved: HashSet<_> = era_validators.non_reserved.iter().cloned().collect();
    let network_non_reserved: HashSet<_> = network_validators.non_reserved.into_iter().collect();
    let network_seats = root_connection
        .get_committee_seats(first_block_in_session)
        .await;

    assert_eq!(reserved, network_reserved);
    assert_eq!(non_reserved, network_non_reserved);
    assert_eq!(seats, network_seats);

    Ok((era_validators, seats, session))
}

pub async fn validators_bond_extra_stakes(config: &Config, additional_stakes: &[Balance]) {
    let node = &config.node;
    let root_connection = config.create_root_connection().await;

    let accounts_keys: Vec<NodeKeys> = get_validators_seeds(config)
        .into_iter()
        .map(|seed| seed.into())
        .collect();
    for (account_keys, additional_stake) in accounts_keys.into_iter().zip(additional_stakes.iter())
    {
        let validator_id = account_from_keypair(account_keys.validator.signer());

        // Additional TOKEN to cover fees
        root_connection
            .transfer_keep_alive(validator_id, *additional_stake + TOKEN, TxStatus::Finalized)
            .await
            .unwrap();
        let stash_connection = SignedConnection::new(node, account_keys.validator).await;
        stash_connection
            .bond_extra_stake(*additional_stake, TxStatus::Finalized)
            .await
            .unwrap();
    }
}
