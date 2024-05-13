use std::sync::Arc;

use sc_client_api::Backend;
use sc_network::{config::Role, NetworkService};
use sc_network_sync::SyncingService;
use sc_rpc::system::Request as RpcRequest;
use sc_service::{build_system_rpc_future, SpawnTaskHandle};
use sc_utils::mpsc::{tracing_unbounded, TracingUnboundedSender};
use sp_runtime::traits::Block;

use crate::{network::build::SPAWN_CATEGORY, ClientForAleph};

/// Spawn the RPC handling service and return the interface for submitting requests to it.
pub fn spawn_rpc_service<B: Block, BE: Backend<B>, C: ClientForAleph<B, BE>>(
    network: Arc<NetworkService<B, B::Hash>>,
    sync_service: Arc<SyncingService<B>>,
    client: Arc<C>,
    spawn_handle: &SpawnTaskHandle,
) -> TracingUnboundedSender<RpcRequest<B>> {
    let (rpcs_for_handling, rpcs_from_user) = tracing_unbounded("mpsc_system_rpc", 10_000);
    spawn_handle.spawn(
        "system-rpc-handler",
        SPAWN_CATEGORY,
        build_system_rpc_future(
            Role::Full,
            network,
            sync_service,
            client,
            rpcs_from_user,
            // We almost always run with bootnodes, and this impacts only one deprecated RPC call, so whatever.
            true,
        ),
    );
    rpcs_for_handling
}
