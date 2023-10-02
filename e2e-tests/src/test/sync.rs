use anyhow::{anyhow, Context};
use log::info;

use crate::{
    config::{setup_test, NodeConfig},
    synthetic_network::{
        await_finalized_blocks, await_new_blocks, execute_synthetic_network_test,
        NodesConnectivityConfiguration,
    },
};

/// Forces a single node to lag behind the rest of the network by disconnecting it with every other
/// peer and then after some time makes it critical for achieving consensus, i.e. it creates a group
/// of connected peers (disconnected with any other peer outside of that group) that includes it and
/// is of minimal size that allows to achieve consensus.
#[tokio::test]
pub async fn one_node_catching_up_and_then_becoming_necessary_for_consensus() -> anyhow::Result<()>
{
    const NUMBER_OF_BLOCKS_TO_WAIT_AFTER_DISCONNECT: u32 = 128;
    const NUMBER_OF_BLOCKS_TO_WAIT_AFTER_RECONNECT: u32 = 32;

    let config = setup_test();
    if config.validator_count < 4 {
        return Err(anyhow!(
            "minimal required number of validators to run this test is 4"
        ));
    }
    let nodes_configs: Vec<_> = config
        .nodes_configs()
        .context("unable to build configuration for test nodes")?
        .collect();

    let mut other_nodes = nodes_configs.clone();
    let selected_node = other_nodes.remove(2);
    let mut disconnected_node = vec![selected_node.clone()];

    let disconnect_configuration =
        NodesConnectivityConfiguration::from(vec![other_nodes.clone(), disconnected_node.clone()]);
    let reconnect_configuration = disconnect_configuration.clone().reconnect();

    let byzantine_set_size = (nodes_configs.len() - 1) / 3;
    let (left_side, right_side) = nodes_configs.as_slice().split_at(byzantine_set_size + 1);

    let mut left_side = left_side.to_vec();
    left_side.retain(|config| config.ip_address() != selected_node.ip_address());
    let mut right_side = right_side.to_vec();
    right_side.append(&mut disconnected_node);
    let final_configuration =
        reconnect_configuration.merge(vec![left_side, right_side.clone()].into());

    perform_test(
        nodes_configs.as_slice(),
        other_nodes.as_slice(),
        right_side.as_slice(),
        disconnect_configuration,
        final_configuration,
        NUMBER_OF_BLOCKS_TO_WAIT_AFTER_DISCONNECT,
        NUMBER_OF_BLOCKS_TO_WAIT_AFTER_RECONNECT,
    )
    .await
}

/// Forces a single node to lag behind the rest of the network. After some time it reconnects all
/// nodes and then checks if all nodes are able to finalize their blocks.
#[tokio::test]
pub async fn one_node_catching_up() -> anyhow::Result<()> {
    const NUMBER_OF_BLOCKS_TO_WAIT_AFTER_DISCONNECT: u32 = 128;
    const NUMBER_OF_BLOCKS_TO_WAIT_AFTER_RECONNECT: u32 = 32;

    let config = setup_test();
    if config.validator_count < 4 {
        return Err(anyhow!(
            "minimal required number of validators to run this test is 4"
        ));
    }
    let nodes_configs: Vec<_> = config
        .nodes_configs()
        .context("unable to build configuration for test nodes")?
        .collect();

    let mut other_nodes = nodes_configs.clone();
    let disconnected_node = vec![other_nodes.remove(2)];

    let disconnect_configuration =
        NodesConnectivityConfiguration::from(vec![other_nodes.clone(), disconnected_node.clone()]);
    let reconnect_configuration = disconnect_configuration.clone().reconnect();

    perform_test(
        nodes_configs.as_slice(),
        other_nodes.as_slice(),
        nodes_configs.as_slice(),
        disconnect_configuration,
        reconnect_configuration,
        NUMBER_OF_BLOCKS_TO_WAIT_AFTER_DISCONNECT,
        NUMBER_OF_BLOCKS_TO_WAIT_AFTER_RECONNECT,
    )
    .await
}

/// First, we divide nodes into two groups where only one contains a quorum. After some time we
/// modify nodes connectivity so the nodes that previously were not included in a quorum now become
/// part of it. Then we check if subset that contains a quorum is still able to finalize new blocks.
#[tokio::test]
pub async fn into_two_groups_and_one_quorum_and_switch_quorum_between_them() -> anyhow::Result<()> {
    const NUMBER_OF_BLOCKS_TO_WAIT: u32 = 32;

    let config = setup_test();
    if config.validator_count < 7 {
        return Err(anyhow!(
            "minimal required number of validators to run this test is 7"
        ));
    }

    let nodes_configs = config
        .nodes_configs()
        .context("unable to build configuration for test nodes")?;

    let (left_side, right_side) = nodes_configs
        .as_slice()
        .split_at((nodes_configs.len() - 1) / 3);

    let (left_side_after_reconnect, right_side_after_reconnect) = nodes_configs
        .as_slice()
        .split_at(nodes_configs.len() - ((nodes_configs.len() - 1) / 3));

    let disconnect_configuration =
        NodesConnectivityConfiguration::from(vec![left_side.to_vec(), right_side.to_vec()]);
    let reconnect_configuration = disconnect_configuration.clone().reconnect();

    let final_configuration = NodesConnectivityConfiguration::from(vec![
        left_side_after_reconnect.to_vec(),
        right_side_after_reconnect.to_vec(),
    ]);
    let final_configuration = reconnect_configuration.merge(final_configuration);

    perform_test(
        nodes_configs.as_slice(),
        nodes_configs.as_slice(),
        left_side_after_reconnect,
        disconnect_configuration,
        final_configuration,
        NUMBER_OF_BLOCKS_TO_WAIT,
        NUMBER_OF_BLOCKS_TO_WAIT,
    )
    .await
}

/// It divides nodes into disjoint groups consisting of two nodes each, then awaits for all these
/// sets to create several new blocks (which shouldn't be finalized), reconnects them and checks if
/// nodes are still able to finalize new blocks.
#[tokio::test]
pub async fn into_multiple_groups_of_two() -> anyhow::Result<()> {
    const NUMBER_OF_BLOCKS_TO_WAIT: u32 = 32;

    let config = setup_test();

    let nodes_configs = config
        .nodes_configs()
        .context("unable to build configuration for test nodes")?;
    if nodes_configs.len() < 2 {
        return Err(anyhow!("not enough nodes in configuration"));
    }

    let groups = nodes_configs.as_slice().chunks(2);
    let groups: Vec<Vec<NodeConfig>> = groups.fold(Vec::new(), |mut result, chunk| {
        if chunk.len() < 2 {
            if let Some(last) = result.last_mut() {
                last.append(&mut chunk.to_vec());
            }
        } else {
            result.push(chunk.to_vec());
        }
        result
    });

    let disconnect_configuration = NodesConnectivityConfiguration::from(groups);
    let reconnect_configuration = disconnect_configuration.clone().reconnect();

    perform_test(
        nodes_configs.as_slice(),
        nodes_configs.as_slice(),
        nodes_configs.as_slice(),
        disconnect_configuration,
        reconnect_configuration,
        NUMBER_OF_BLOCKS_TO_WAIT,
        NUMBER_OF_BLOCKS_TO_WAIT,
    )
    .await
}

/// Tests if nodes are able to proceed after we divide them into two ~equal size disjoint sets. More
/// precisely, it divides nodes into two halves, awaits for both sets to create several new blocks
/// (which shouldn't be finalized), reconnects them and checks if nodes are still able to finalize
/// new blocks.
#[tokio::test]
pub async fn into_two_equal_size_groups_with_no_quorum() -> anyhow::Result<()> {
    const NUMBER_OF_BLOCKS_TO_WAIT: u32 = 32;

    let config = setup_test();

    let nodes_configs = config
        .nodes_configs()
        .context("unable to build configuration for test nodes")?;

    let (left_side, right_side) = nodes_configs.as_slice().split_at(nodes_configs.len() / 2);

    let disconnect_configuration =
        NodesConnectivityConfiguration::from(vec![left_side.to_vec(), right_side.to_vec()]);
    let reconnect_configuration = disconnect_configuration.clone().reconnect();

    perform_test(
        nodes_configs.as_slice(),
        nodes_configs.as_slice(),
        nodes_configs.as_slice(),
        disconnect_configuration,
        reconnect_configuration,
        NUMBER_OF_BLOCKS_TO_WAIT,
        NUMBER_OF_BLOCKS_TO_WAIT,
    )
    .await
}

/// We divide nodes into two disjoint sets where on of them contains a quorum, await for nodes in
/// both sets to create several new blocks (only one them should be able to finalize), reconnect all
/// nodes and then check if nodes are still able to finalize new blocks.
#[tokio::test]
pub async fn into_two_groups_one_with_quorum() -> anyhow::Result<()> {
    const NUMBER_OF_BLOCKS_TO_WAIT: u32 = 32;

    let config = setup_test();
    if config.validator_count < 7 {
        return Err(anyhow!(
            "provided test-network is to small ({0}), should be >= 7",
            config.validator_count,
        ));
    }

    let nodes_configs = config
        .nodes_configs()
        .context("unable to build configuration for test nodes")?;

    let byzantine_set_size = (nodes_configs.len() - 1) / 3;
    let (left_side, right_side) = nodes_configs.as_slice().split_at(byzantine_set_size);

    let disconnect_configuration =
        NodesConnectivityConfiguration::from(vec![left_side.to_vec(), right_side.to_vec()]);
    let reconnect_configuration = disconnect_configuration.clone().reconnect();

    perform_test(
        nodes_configs.as_slice(),
        nodes_configs.as_slice(),
        nodes_configs.as_slice(),
        disconnect_configuration,
        reconnect_configuration,
        NUMBER_OF_BLOCKS_TO_WAIT,
        NUMBER_OF_BLOCKS_TO_WAIT,
    )
    .await
}

async fn perform_test(
    all_nodes_to_check: impl IntoIterator<Item = &NodeConfig>,
    nodes_to_check_after_disconnect: impl IntoIterator<Item = &NodeConfig> + Clone,
    nodes_to_check_after_reconfigure: impl IntoIterator<Item = &NodeConfig> + Clone,
    disconnect_configuration: NodesConnectivityConfiguration,
    reconnect_configuration: NodesConnectivityConfiguration,
    blocks_to_wait_after_disconnect: u32,
    blocks_to_wait_after_reconnect: u32,
) -> anyhow::Result<()> {
    execute_synthetic_network_test(all_nodes_to_check, async move {
        // check the finalization first
        await_finalized_blocks(nodes_to_check_after_disconnect.clone(), 0, 2).await?;

        info!("Configuring network connectivity");
        disconnect_configuration.commit().await?;

        let best_seen_block = await_new_blocks(
            nodes_to_check_after_disconnect,
            blocks_to_wait_after_disconnect,
        )
        .await?;

        info!("Re-configuring network connectivity");
        reconnect_configuration.commit().await?;

        await_finalized_blocks(
            nodes_to_check_after_reconfigure,
            best_seen_block,
            blocks_to_wait_after_reconnect,
        )
        .await
    })
    .await
}
