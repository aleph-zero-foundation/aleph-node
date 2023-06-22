use aleph_client::{
    utility::BlocksApi,
    waiting::{AlephWaiting, BlockStatus},
    SignedConnection,
};
use anyhow::anyhow;
use futures::future::{join_all, try_join_all};
use log::info;
use synthetic_link::SyntheticNetworkClient;

use crate::config::Config;

pub type Milliseconds = u64;

pub const OUT_LATENCY: Milliseconds = 200;

pub async fn set_out_latency(milliseconds: Milliseconds, synthetic_url: String) {
    info!(
        "setting out-latency of node {} to {}ms",
        synthetic_url, milliseconds
    );
    info!("creating an http client for url {}", synthetic_url);
    let mut client = SyntheticNetworkClient::new(synthetic_url);
    let mut config = client
        .load_config()
        .await
        .expect("we should be able to download config of the synthetic-network ");

    config.default_link.egress.latency = milliseconds;

    client
        .commit_config(&config)
        .await
        .expect("unable to commit network configuration");
}

async fn wait_for_further_finalized_blocks(
    connection: &SignedConnection,
    blocks_to_wait: u32,
) -> anyhow::Result<()> {
    let finalized = connection.get_finalized_block_hash().await?;
    let finalized_number = connection
        .get_block_number(finalized)
        .await?
        .ok_or(anyhow!(
            "Failed to retrieve block number for hash {finalized:?}"
        ))?;
    let block_number_to_wait = finalized_number + blocks_to_wait;
    info!(
        "Current finalized block #{}, waiting for block #{}",
        finalized_number, block_number_to_wait
    );
    connection
        .wait_for_block(|n| n > block_number_to_wait, BlockStatus::Finalized)
        .await;
    Ok(())
}

pub async fn test_latency_template_test(
    config: &Config,
    validator_count: usize,
    out_latency: Milliseconds,
) -> anyhow::Result<()> {
    let connections = config.create_signed_connections().await;
    join_all(
        config
            .synthetic_network_urls()
            .into_iter()
            .take(validator_count)
            .map(|synthetic_url| set_out_latency(out_latency, synthetic_url)),
    )
    .await;
    info!("Waiting for session 1");
    join_all(
        connections
            .iter()
            .map(|connection| connection.wait_for_session(1, BlockStatus::Finalized)),
    )
    .await;
    let blocks_to_wait_in_first_session = 30;
    info!(
        "Waiting for {} finalized blocks in sesssion 1 to make sure initial unit collection works.",
        blocks_to_wait_in_first_session
    );
    try_join_all(connections.iter().map(|connection| {
        wait_for_further_finalized_blocks(connection, blocks_to_wait_in_first_session)
    }))
    .await?;
    Ok(())
}
