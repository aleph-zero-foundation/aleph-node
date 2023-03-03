use aleph_client::{
    utility::BlocksApi,
    waiting::{AlephWaiting, BlockStatus},
};
use anyhow::anyhow;

use crate::config::setup_test;

#[tokio::test]
pub async fn finalization() -> anyhow::Result<()> {
    let config = setup_test();
    let connection = config.create_root_connection().await;

    let finalized = connection.get_finalized_block_hash().await?;
    let finalized_number = connection
        .get_block_number(finalized)
        .await?
        .ok_or(anyhow!(
            "Failed to retrieve block number for hash {finalized:?}"
        ))?;

    connection
        .wait_for_block(|n| n > finalized_number, BlockStatus::Finalized)
        .await;

    Ok(())
}
