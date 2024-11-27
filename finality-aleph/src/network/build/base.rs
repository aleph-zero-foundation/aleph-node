use std::sync::Arc;

use libp2p::{core::StreamMuxer, PeerId, Transport};
use sc_client_api::Backend;
use sc_network::{
    config::{
        FullNetworkConfiguration, NetworkConfiguration, NonDefaultSetConfig,
        Params as NetworkParams, ProtocolId, Role,
    },
    error::Error as NetworkError,
    peer_store::PeerStore,
    transport::NetworkConfig,
    NetworkService, NetworkWorker,
};
use sc_network_light::light_client_requests::handler::LightClientRequestHandler;
use sc_network_sync::state_request_handler::StateRequestHandler;
use sc_network_transactions::TransactionsHandlerPrototype;
use sc_service::SpawnTaskHandle;
use sp_runtime::traits::{Block, Header};
use substrate_prometheus_endpoint::Registry;

use crate::{
    network::build::{
        own_protocols::Networks, transactions::build_transactions_prototype, SPAWN_CATEGORY,
    },
    BlockHash, BlockNumber, ClientForAleph,
};

fn spawn_state_request_handler<B: Block, BE: Backend<B>, C: ClientForAleph<B, BE>>(
    full_network_config: &mut FullNetworkConfiguration,
    protocol_id: &ProtocolId,
    client: Arc<C>,
    spawn_handle: &SpawnTaskHandle,
) {
    let num_peer_hint = full_network_config
        .network_config
        .default_peers_set_num_full as usize
        + full_network_config
            .network_config
            .default_peers_set
            .reserved_nodes
            .len();
    let (service, protocol_config) =
    // The None is the fork id, which we don't have.
        StateRequestHandler::new(protocol_id, None, client, num_peer_hint);
    spawn_handle.spawn("state-request-handler", SPAWN_CATEGORY, service.run());
    full_network_config.add_request_response_protocol(protocol_config);
}

fn spawn_light_client_request_handler<B: Block, BE: Backend<B>, C: ClientForAleph<B, BE>>(
    full_network_config: &mut FullNetworkConfiguration,
    protocol_id: &ProtocolId,
    client: Arc<C>,
    spawn_handle: &SpawnTaskHandle,
) {
    let (handler, protocol_config) =
    // The None is the fork id, which we don't have.
        LightClientRequestHandler::new(protocol_id, None, client.clone());
    spawn_handle.spawn(
        "light-client-request-handler",
        SPAWN_CATEGORY,
        handler.run(),
    );
    full_network_config.add_request_response_protocol(protocol_config);
}

type BaseNetworkOutput<B> = (
    Arc<NetworkService<B, <B as Block>::Hash>>,
    Networks,
    TransactionsHandlerPrototype,
);

/// Create a base network with all the protocols already included. Also spawn (almost) all the necessary services.
pub fn network<B, BE, C, T, SM>(
    network_config: &NetworkConfiguration,
    transport_builder: impl FnOnce(NetworkConfig) -> T,
    protocol_id: ProtocolId,
    client: Arc<C>,
    spawn_handle: &SpawnTaskHandle,
    base_protocol_config: NonDefaultSetConfig,
    metrics_registry: Option<Registry>,
) -> Result<BaseNetworkOutput<B>, NetworkError>
where
    B: Block<Hash = BlockHash>,
    B::Header: Header<Number = BlockNumber>,
    BE: Backend<B>,
    C: ClientForAleph<B, BE>,
    T: Transport<Output = (PeerId, SM)> + Send + Unpin + 'static,
    T::Dial: Send,
    T::ListenerUpgrade: Send,
    T::Error: Send + Sync,
    SM: StreamMuxer + Unpin + Send + 'static,
    SM::Substream: Unpin + Send,
    SM::Error: Send + Sync,
{
    let mut full_network_config = FullNetworkConfiguration::new(network_config);
    let genesis_hash = client
        .hash(0)
        .ok()
        .flatten()
        .expect("Genesis block exists.");
    let networks = Networks::new(&mut full_network_config, &genesis_hash);

    spawn_state_request_handler(
        &mut full_network_config,
        &protocol_id,
        client.clone(),
        spawn_handle,
    );
    spawn_light_client_request_handler(
        &mut full_network_config,
        &protocol_id,
        client.clone(),
        spawn_handle,
    );
    let transactions_prototype =
        build_transactions_prototype(&mut full_network_config, &protocol_id, genesis_hash);

    let peer_store_service = PeerStore::new(
        full_network_config
            .network_config
            .boot_nodes
            .iter()
            .map(|bootnode| bootnode.peer_id)
            .collect(),
    );
    let peer_store = peer_store_service.handle();
    spawn_handle.spawn("peer-store", SPAWN_CATEGORY, peer_store_service.run());

    let network_params = NetworkParams::<B> {
        role: Role::Full,
        executor: {
            let spawn_handle = spawn_handle.clone();
            Box::new(move |fut| {
                spawn_handle.spawn("libp2p-node", SPAWN_CATEGORY, fut);
            })
        },
        network_config: full_network_config,
        peer_store,
        genesis_hash,
        protocol_id: protocol_id.clone(),
        fork_id: None,
        metrics_registry: metrics_registry.clone(),
        // The names are silly, but that's substrate's fault.
        block_announce_config: base_protocol_config,
    };

    let network_service =
        NetworkWorker::new_with_custom_transport(network_params, transport_builder)?;
    let network = network_service.service().clone();
    spawn_handle.spawn_blocking("network-worker", SPAWN_CATEGORY, network_service.run());
    Ok((network, networks, transactions_prototype))
}
