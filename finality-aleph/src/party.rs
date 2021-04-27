use crate::{
    environment::Environment, network, network::ConsensusNetwork, EpochId, NodeId, SpawnHandle,
};

use futures::{channel::mpsc, SinkExt};
use rush::Consensus;
use sc_client_api::backend::Backend;
use sp_consensus::SelectChain;
use sp_core::{Blake2Hasher, Hasher, H256};
use sp_runtime::traits::Block;

pub struct AlephParams<N, C, SC> {
    pub config: crate::AlephConfig<N, C, SC>,
}

pub async fn run_consensus_party<B, N, C, BE, SC>(aleph_params: AlephParams<N, C, SC>)
where
    B: Block,
    N: network::Network<B> + 'static,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
{
    // TODO now it runs just a single instance of consensus but later it will
    // orchestrate managing multiple instances for differents epoch
    let AlephParams {
        config:
            crate::AlephConfig {
                network,
                consensus_config,
                client,
                select_chain,
                spawn_handle,
                auth_keystore,
                authorities,
            },
    } = aleph_params;
    let network = ConsensusNetwork::new(network, "/cardinals/aleph/1");
    let spawn_handle: SpawnHandle = spawn_handle.into();

    let (net_command_tx, net_command_rx) = mpsc::unbounded();
    let task = {
        let network = network.clone();
        async move { network.run(net_command_rx).await }
    };

    spawn_handle.0.spawn("aleph/network", task);

    let epoch_id = EpochId(0);
    let network_event_rx = network.start_epoch(epoch_id, authorities.clone());
    let hashing = Blake2Hasher::hash;

    let (notification_in_tx, notification_in_rx) = mpsc::unbounded();
    let (notification_out_tx, notification_out_rx) = mpsc::unbounded();
    // Making `rush::Consensus` accept only tokio sender sinks seems a questionable choice...
    let (order_tx, order_rx) = tokio::sync::mpsc::unbounded_channel();

    let env: Environment<B, H256, C, BE, SC> = Environment::new(
        client,
        select_chain,
        notification_in_tx,
        notification_out_rx,
        net_command_tx,
        network_event_rx,
        order_rx,
        authorities,
        auth_keystore,
        hashing,
        epoch_id,
    );

    let consensus: Consensus<H256, NodeId> = Consensus::new(
        consensus_config,
        notification_in_rx,
        notification_out_tx.sink_map_err(|e| e.into()),
        order_tx,
        hashing,
    );

    rush::SpawnHandle::spawn(&spawn_handle.clone(), "aleph/environment", env.run_epoch());
    log::debug!(target: "afa", "Environment has started");

    let (_exit, exit) = tokio::sync::oneshot::channel();
    log::debug!(target: "afa", "Consensus party has started");
    consensus.run(spawn_handle, exit).await;
    log::debug!(target: "afa", "Consensus party has stopped");
}
