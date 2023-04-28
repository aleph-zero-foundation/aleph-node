//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]

use std::sync::Arc;

use aleph_runtime::{
    opaque::{Block, Header},
    AccountId, Balance, Index,
};
use finality_aleph::{Justification, JustificationTranslator};
use futures::channel::mpsc;
use jsonrpsee::RpcModule;
pub use sc_rpc_api::DenyUnsafe;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};

/// Full client dependencies.
pub struct FullDeps<C, P, JT> {
    /// The client instance to use.
    pub client: Arc<C>,
    /// Transaction pool instance.
    pub pool: Arc<P>,
    /// Whether to deny unsafe calls
    pub deny_unsafe: DenyUnsafe,
    pub import_justification_tx: mpsc::UnboundedSender<Justification<Header>>,
    pub justification_translator: JT,
}

/// Instantiate all full RPC extensions.
pub fn create_full<C, P, JT>(
    deps: FullDeps<C, P, JT>,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
    C: ProvideRuntimeApi<Block>,
    C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError> + 'static,
    C: Send + Sync + 'static,
    C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
    C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
    C::Api: BlockBuilder<Block>,
    P: TransactionPool + 'static,
    JT: JustificationTranslator<Header> + Send + Sync + Clone + 'static,
{
    use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
    use substrate_frame_rpc_system::{System, SystemApiServer};

    let mut module = RpcModule::new(());
    let FullDeps {
        client,
        pool,
        deny_unsafe,
        import_justification_tx,
        justification_translator,
    } = deps;

    module.merge(System::new(client.clone(), pool, deny_unsafe).into_rpc())?;

    module.merge(TransactionPayment::new(client).into_rpc())?;

    use crate::aleph_node_rpc::{AlephNode, AlephNodeApiServer};
    module.merge(AlephNode::new(import_justification_tx, justification_translator).into_rpc())?;

    Ok(module)
}
