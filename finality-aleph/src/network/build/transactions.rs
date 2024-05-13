use std::sync::Arc;

use sc_client_api::Backend;
use sc_network::{
    config::{FullNetworkConfiguration, ProtocolId},
    error::Error as NetworkError,
    NetworkService,
};
use sc_network_sync::SyncingService;
use sc_network_transactions::{TransactionsHandlerController, TransactionsHandlerPrototype};
use sc_service::{SpawnTaskHandle, TransactionPoolAdapter};
use sc_transaction_pool_api::TransactionPool;
use sp_runtime::traits::Block;
use substrate_prometheus_endpoint::Registry;

use crate::{network::build::SPAWN_CATEGORY, BlockHash, ClientForAleph};

/// Build a transaction prototype, that can later be used to build the transaction handler,
/// and update the network config with the appropriate protocol.
pub fn build_transactions_prototype(
    full_network_config: &mut FullNetworkConfiguration,
    protocol_id: &ProtocolId,
    genesis_hash: BlockHash,
) -> TransactionsHandlerPrototype {
    let (prototype, protocol_config) =
        TransactionsHandlerPrototype::new(protocol_id.clone(), genesis_hash, None);
    full_network_config.add_notification_protocol(protocol_config);
    prototype
}

/// Spawn the transaction handler and return an interface for interacting with it.
pub fn spawn_transaction_handler<
    TP: TransactionPool + 'static,
    BE: Backend<TP::Block>,
    C: ClientForAleph<TP::Block, BE>,
>(
    network: Arc<NetworkService<TP::Block, <TP::Block as Block>::Hash>>,
    sync_service: Arc<SyncingService<TP::Block>>,
    client: Arc<C>,
    transaction_pool: Arc<TP>,
    transactions_prototype: TransactionsHandlerPrototype,
    metrics_registry: Option<&Registry>,
    spawn_handle: &SpawnTaskHandle,
) -> Result<TransactionsHandlerController<TP::Hash>, NetworkError> {
    let (transaction_service, transaction_interface) = transactions_prototype.build(
        network,
        sync_service,
        Arc::new(TransactionPoolAdapter::new(transaction_pool, client)),
        metrics_registry,
    )?;
    spawn_handle.spawn(
        "network-transactions-handler",
        SPAWN_CATEGORY,
        transaction_service.run(),
    );
    Ok(transaction_interface)
}
