use crate::{
    data_io::{BlockFinalizer, DataIO},
    hash, network,
    network::ConsensusNetwork,
    KeyBox, SessionId, SpawnHandle,
};

use futures::channel::mpsc;
use log::debug;
use sc_client_api::backend::Backend;
use sp_consensus::SelectChain;
use sp_runtime::traits::{BlakeTwo256, Block};

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
    // orchestrate managing multiple instances for differents session
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
    let session_manager = network.session_manager();
    let spawn_handle: SpawnHandle = spawn_handle.into();

    let task = async move { network.run().await };
    spawn_handle.0.spawn("aleph/network", task);
    debug!(target: "afa", "Consensus network has started.");

    let session_id = SessionId(0);
    let id = consensus_config.node_id;
    let session_network = session_manager.start_session(session_id, authorities.clone());
    let (ordered_batch_tx, ordered_batch_rx) = mpsc::unbounded();
    let block_finalizer = BlockFinalizer::new(client, auth_keystore.clone(), ordered_batch_rx);
    let data_io = DataIO {
        select_chain,
        ordered_batch_tx,
    };
    let keybox = KeyBox {
        id,
        auth_keystore,
        authorities,
    };

    let task = async move { block_finalizer.run().await };
    spawn_handle.0.spawn("aleph/finalizer", task);
    debug!(target: "afa", "Block finalizer has started.");

    let (_exit, exit) = futures::channel::oneshot::channel();
    let member = rush::Member::<hash::Wrapper<BlakeTwo256>, _, _, _, _>::new(
        data_io,
        &keybox,
        session_network,
        consensus_config,
    );

    debug!(target: "afa", "Consensus party has started");
    member.run_session(spawn_handle, exit).await;

    debug!(target: "afa", "Consensus party has stopped");
}
