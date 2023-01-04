use std::cmp::max;

use aleph_client::{
    pallets::session::SessionApi,
    waiting::{AlephWaiting, BlockStatus},
};
use log::info;

use crate::{config::setup_test, synthetic_network::set_out_latency};

const OUT_LATENCY: u64 = 500;

/// Test if nodes are able to proceed despite high latency. More precisely, it first awaits predefined number of sessions, sets
/// egress-latency for each node using same value (default is 500 milliseconds) and verifies if after it was able to proceed two
/// more sessions.
#[tokio::test]
pub async fn high_out_latency_for_all() -> anyhow::Result<()> {
    let config = setup_test();
    let out_latency = config.test_case_params.out_latency.unwrap_or(OUT_LATENCY);

    let connections = config.create_signed_connections().await;
    info!("waiting for at least session 3");
    for connection in &connections {
        if connection.get_session(None).await < 3 {
            connection.wait_for_session(3, BlockStatus::Finalized).await;
        }
    }

    info!("setting out-latency");
    for synthetic_url in config.synthetic_network_urls() {
        info!(
            "setting out-latency of node {} to {}",
            synthetic_url, out_latency
        );
        set_out_latency(out_latency, synthetic_url).await;
    }

    let mut max_session = 0;
    for connection in &connections {
        let node_session = connection.get_session(None).await;
        max_session = max(max_session, node_session);
    }
    info!("current session is {}", max_session);

    for connection in connections {
        connection
            .wait_for_session(max_session + 2, BlockStatus::Finalized)
            .await;
    }
    Ok(())
}

/// Test if nodes are able to proceed despite high latency, but set only for a subset of nodes. More precisely, it first awaits
/// predefined number of sessions, sets egress-latency for 1/3n of nodes using same value (default is 500 milliseconds) and
/// verifies if after it was able to proceed two more sessions.
#[tokio::test]
pub async fn high_out_latency_for_each_quorum() -> anyhow::Result<()> {
    let config = setup_test();
    let out_latency = config.test_case_params.out_latency.unwrap_or(OUT_LATENCY);

    let connections = config.create_signed_connections().await;
    info!("waiting for at least session 3");
    for connection in &connections {
        if connection.get_session(None).await < 3 {
            connection.wait_for_session(3, BlockStatus::Finalized).await;
        }
    }

    info!("setting out-latency");
    for synthetic_url in config
        .synthetic_network_urls()
        .into_iter()
        .take(((config.validator_count - 1) / 3 + 1) as usize)
    {
        info!(
            "setting out-latency of node {} to {}",
            synthetic_url, out_latency
        );
        set_out_latency(out_latency, synthetic_url).await;
    }

    let mut max_session = 0;
    for connection in &connections {
        let node_session = connection.get_session(None).await;
        max_session = max(max_session, node_session);
    }
    info!("current session is {}", max_session);

    info!("waiting for session {}", max_session + 2);
    for connection in connections {
        connection
            .wait_for_session(max_session + 2, BlockStatus::Finalized)
            .await;
    }
    Ok(())
}
