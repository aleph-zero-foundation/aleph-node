use aleph_client::{pallets::aleph::AlephApi, utility::BlocksApi, Connection};
use log::info;
use primitives::{BlockNumber, Version};

pub async fn check_finality_version_at_block(
    connection: &Connection,
    block_number: BlockNumber,
    expected_version: Version,
) {
    info!(
        "Checking current session finality version for block {}",
        block_number
    );
    let block_hash = connection
        .get_block_hash(block_number)
        .await
        .expect("Should have been able to get a block hash!");
    let finality_version = connection.finality_version(block_hash).await;
    assert_eq!(finality_version, expected_version);
}

pub async fn check_next_session_finality_version_at_block(
    connection: &Connection,
    block_number: BlockNumber,
    expected_version: Version,
) {
    info!(
        "Checking next session finality version for block {}",
        block_number
    );
    let block_hash = connection
        .get_block_hash(block_number)
        .await
        .expect("Should have been able to get a block hash!");
    let next_finality_version = connection.next_session_finality_version(block_hash).await;
    assert_eq!(next_finality_version, expected_version);
}
