use std::{thread::sleep, time::Duration};

use aleph_client::{
    pallets::{elections::ElectionsSudoApi, session::SessionApi},
    primitives::CommitteeSeats,
    utility::BlocksApi,
    waiting::{BlockStatus, WaitingExt},
    AccountId, AsConnection, Pair, SignedConnection, TxStatus,
};
use log::info;

use crate::{
    accounts::{get_validator_seed, get_validators_keys, NodeKeys},
    config::setup_test,
    rewards::set_invalid_keys_for_validator,
    validators::validator_address,
};

/// approx. time needed for 5 out of 7 block producers to do 3 sessions.
const SLEEP_DURATION: Duration = Duration::from_secs(130);

async fn prepare_test() -> anyhow::Result<()> {
    let config = setup_test();

    let accounts = get_validators_keys(config);
    let connection = config.create_root_connection().await;

    let new_validators: Vec<AccountId> = accounts
        .iter()
        .map(|pair| pair.signer().public().into())
        .collect();

    let seats = CommitteeSeats {
        reserved_seats: 3,
        non_reserved_seats: 4,
        non_reserved_finality_seats: 1,
    };

    info!("validators: {:?}", new_validators);

    connection
        .change_validators(
            Some(new_validators[0..3].to_vec()),
            Some(new_validators[3..].to_vec()),
            Some(seats),
            TxStatus::InBlock,
        )
        .await?;
    connection.wait_for_n_eras(1, BlockStatus::Finalized).await;

    Ok(())
}

pub async fn disable_validators(indexes: &[u32]) -> anyhow::Result<()> {
    info!("Disabling {:?} validators", indexes);
    let mut connections = vec![];
    let address = validator_address(0);

    for &index in indexes {
        let validator_seed = get_validator_seed(index);
        let validator = NodeKeys::from(validator_seed).validator;

        connections.push(SignedConnection::new(&address, validator).await);
    }

    set_invalid_keys_for_validator(connections).await
}

/// Setup finality committee to be of constructed from 3 reserved nodes and 1 non-reserved node.
/// 7 nodes are responsible for creating blocks (3 reserved, 4 nonreserved). We first kill `validators`
/// and then check whether finalization stopped and after a time also block-production. This means the killed
/// validators were in the finality committee (like the should be).
async fn split_disable(validators: &[u32]) -> anyhow::Result<()> {
    let config = setup_test();
    let root_connection = config.create_root_connection().await;
    let connection = root_connection.as_connection();
    prepare_test().await?;

    // For each reserved node disable it and check that block finalization stopped.
    // To check that we check that at most 2 sessions passed after disabling - we have limit of 20 blocks
    // created after last finalized block.
    info!(
        "Testing if #{:?} reserved validators are in finalization committee",
        validators
    );
    disable_validators(validators).await?;
    let session_before = connection.get_session(None).await;
    let block_before = connection
        .get_best_block()
        .await?
        .expect("there should be some block");
    sleep(SLEEP_DURATION);
    let session_after = connection.get_session(None).await;
    let block_after = connection
        .get_best_block()
        .await?
        .expect("there should be some block");
    assert!(
        session_before + 2 >= session_after,
        "too many sessions passed, sessions that passed: {session_before} to {session_after}"
    );
    // at least some blocks were produced after disabling
    assert!(
        block_after > block_before + 10,
        "not enough blocks produced: blocks created: {block_before} to {block_after}"
    );

    Ok(())
}

#[tokio::test]
/// Check if reserved node-0 and node-1 are in the finality committee
async fn split_test_reserved_01() -> anyhow::Result<()> {
    split_disable(&[0, 1]).await
}

#[tokio::test]
/// Check if reserved node-1 and node-2 are in the finality committee
async fn split_test_reserved_12() -> anyhow::Result<()> {
    split_disable(&[1, 2]).await
}

#[tokio::test]
/// Check if reserved node-0 and node-2 are in the finality committee
async fn split_test_reserved_02() -> anyhow::Result<()> {
    split_disable(&[0, 2]).await
}

#[tokio::test]
/// Check if chain runs smoothly while finality committee splits from block producers for couple of eras
async fn split_test_success_without_any_deads() -> anyhow::Result<()> {
    prepare_test().await?;

    let connection = setup_test().get_first_signed_connection().await;
    connection.wait_for_n_eras(2, BlockStatus::Finalized).await;

    Ok(())
}

#[tokio::test]
/// Check if chain runs smoothly while one member of finality committee is dead for couple of eras
async fn split_test_success_with_one_dead() -> anyhow::Result<()> {
    prepare_test().await?;

    let connection = setup_test().get_first_signed_connection().await;
    disable_validators(&[0]).await?;
    connection.wait_for_n_eras(1, BlockStatus::Finalized).await;

    Ok(())
}

#[tokio::test]
/// Check if chain runs 'kinda'-smoothly while at most one of the finality committee member is dead.
/// Here, we kill all of the non-reserved nodes. This will slow down block production but won't kill
/// the finalization because 3 out of 4 nodes in finality committee are from reserved set.
async fn split_test_success_with_all_non_reserved_dead() -> anyhow::Result<()> {
    prepare_test().await?;

    let connection = setup_test().get_first_signed_connection().await;
    // kill all non-reserved
    disable_validators(&[3, 4, 5, 6]).await?;
    // 5 session, so all of the non-reserved nodes have enough time to be in the finality committee
    connection
        .wait_for_n_sessions(5, BlockStatus::Finalized)
        .await;

    Ok(())
}
