use std::{thread::sleep, time::Duration};

use aleph_client::{
    pallets::{
        aleph::{AlephApi, AlephRpc, AlephSudoApi},
        elections::ElectionsSudoApi,
        staking::StakingApi,
    },
    primitives::CommitteeSeats,
    utility::BlocksApi,
    waiting::{BlockStatus, WaitingExt},
    AccountId, AlephKeyPair, AsConnection, BlockHash, Pair, TxStatus,
};

use crate::{
    accounts::get_validators_keys, config::setup_test, test::committee_split::disable_validators,
};

#[tokio::test]
async fn set_emergency_finalizer_test() -> anyhow::Result<()> {
    let config = setup_test();
    let (finalizer, _seed) = AlephKeyPair::generate();
    let public = finalizer.public().0;
    let root = config.create_root_connection().await;
    let current_finalizer = root.as_connection().emergency_finalizer(None).await;

    assert_ne!(current_finalizer, Some(public));

    root.set_emergency_finalizer(finalizer.public().into(), TxStatus::Finalized)
        .await?;
    root.wait_for_n_sessions(2, BlockStatus::Finalized).await;

    let current_finalizer = root.as_connection().emergency_finalizer(None).await;
    assert_eq!(current_finalizer, Some(public));
    Ok(())
}

async fn finalize_from_to<C: AsConnection + Sync>(
    connection: &C,
    from: BlockHash,
    to: BlockHash,
    finalizer: AlephKeyPair,
) -> anyhow::Result<()> {
    let from = connection.get_block_number(from).await?.unwrap();
    let to = connection.get_block_number(to).await?.unwrap();

    for i in (from + 1)..=to {
        let hash = connection.get_block_hash(i).await?.unwrap();
        connection.emergency_finalize(i, hash, finalizer).await?
    }
    Ok(())
}

async fn setup() -> anyhow::Result<AlephKeyPair> {
    let config = setup_test();
    let (finalizer, _seed) = AlephKeyPair::generate();
    let connection = config.create_root_connection().await;

    connection
        .set_emergency_finalizer(finalizer.public().into(), TxStatus::Finalized)
        .await?;
    connection
        .wait_for_n_sessions(2, BlockStatus::Finalized)
        .await;

    let accounts = get_validators_keys(config);

    let new_validators: Vec<AccountId> = accounts
        .iter()
        .map(|pair| pair.signer().public().into())
        .collect();

    let seats = CommitteeSeats {
        reserved_seats: 4,
        non_reserved_seats: 0,
        non_reserved_finality_seats: 0,
    };

    connection
        .change_validators(
            Some(new_validators[0..4].to_vec()),
            Some(vec![]),
            Some(seats.clone()),
            TxStatus::InBlock,
        )
        .await?;
    connection.wait_for_n_eras(1, BlockStatus::Finalized).await;
    connection
        .change_validators(
            Some(new_validators[2..].to_vec()),
            Some(vec![]),
            Some(seats),
            TxStatus::InBlock,
        )
        .await?;
    disable_validators(&[0, 1]).await?;

    Ok(finalizer)
}

/// Tests emergency finalizer. Runs on 6 nodes 0-5.
/// * Setup finalizer
/// * setup 0-3 to be validators in the next era
/// * setup 2-5 to be validators in the next next era
/// * disable 0-1.
/// * wait for next era
/// * check if finalization stopped
/// * use finalizer to advance into next-next era
/// * check if finalization resumed
#[tokio::test]
async fn chain_dead_scenario() -> anyhow::Result<()> {
    let config = setup_test();
    let finalizer = setup().await?;
    let connection = config.create_root_connection().await;

    let last_best_block_before = connection.get_best_block().await?.unwrap();
    sleep(Duration::from_secs(40));
    let mut last_finalized = connection.get_finalized_block_hash().await?;
    let last_best_block = connection.get_best_block().await?.unwrap();

    assert!(
        last_best_block - last_best_block_before <= 20,
        "at most 20 blocks can be created after finalization stops. from {last_best_block_before} to {last_best_block}"
    );
    let current_era = connection.get_active_era(Some(last_finalized)).await;

    // use finalizer to advance into the next era
    while current_era == connection.get_active_era(Some(last_finalized)).await {
        let last_best_block = connection.get_best_block().await?.unwrap();
        let last_best_block = connection.get_block_hash(last_best_block).await?.unwrap();
        finalize_from_to(&connection, last_finalized, last_best_block, finalizer).await?;
        sleep(Duration::from_secs(40));

        last_finalized = connection.get_finalized_block_hash().await?;
    }

    // chain resumes after emergency finalizer fixes the issue
    connection
        .wait_for_n_sessions(1, BlockStatus::Finalized)
        .await;

    Ok(())
}
