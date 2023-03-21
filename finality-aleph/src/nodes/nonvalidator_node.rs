use aleph_primitives::BlockNumber;
use log::{debug, error};
use sc_client_api::Backend;
use sc_network_common::ExHashT;
use sp_consensus::SelectChain;
use sp_runtime::traits::{Block, Header};

use crate::{
    nodes::{setup_justification_handler, JustificationParams},
    session_map::{AuthorityProviderImpl, FinalityNotifierImpl, SessionMapUpdater},
    AlephConfig, BlockchainBackend,
};

pub async fn run_nonvalidator_node<B, H, C, BB, BE, SC>(aleph_config: AlephConfig<B, H, C, SC, BB>)
where
    B: Block,
    B::Header: Header<Number = BlockNumber>,
    H: ExHashT,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B>,
    BE: Backend<B> + 'static,
    BB: BlockchainBackend<B> + Send + 'static,
    SC: SelectChain<B> + 'static,
{
    let AlephConfig {
        network,
        client,
        blockchain_backend,
        metrics,
        session_period,
        millisecs_per_block,
        justification_rx,
        spawn_handle,
        ..
    } = aleph_config;
    let map_updater = SessionMapUpdater::new(
        AuthorityProviderImpl::new(client.clone()),
        FinalityNotifierImpl::new(client.clone()),
        session_period,
    );
    let session_authorities = map_updater.readonly_session_map();
    spawn_handle.spawn("aleph/updater", None, async move {
        debug!(target: "aleph-party", "SessionMapUpdater has started.");
        map_updater.run().await
    });
    let (_, handler_task) = setup_justification_handler(JustificationParams {
        justification_rx,
        network,
        client,
        blockchain_backend,
        metrics,
        session_period,
        millisecs_per_block,
        session_map: session_authorities,
    });

    debug!(target: "aleph-party", "JustificationHandler has started.");
    handler_task.await;
    error!(target: "aleph-party", "JustificationHandler finished.");
}
