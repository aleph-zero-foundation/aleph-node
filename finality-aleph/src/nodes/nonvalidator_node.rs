use log::{debug, error};
use sc_client_api::Backend;
use sc_network::ExHashT;
use sp_consensus::SelectChain;
use sp_runtime::traits::Block;

use crate::{
    nodes::{setup_justification_handler, JustificationParams},
    session_map::{AuthorityProviderImpl, FinalityNotificatorImpl, SessionMapUpdater},
    AlephConfig,
};

pub async fn run_nonvalidator_node<B, H, C, BE, SC>(aleph_config: AlephConfig<B, H, C, SC>)
where
    B: Block,
    H: ExHashT,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B>,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
{
    let AlephConfig {
        network,
        client,
        metrics,
        session_period,
        millisecs_per_block,
        justification_rx,
        spawn_handle,
        ..
    } = aleph_config;
    let map_updater = SessionMapUpdater::<_, _, B>::new(
        AuthorityProviderImpl::new(client.clone()),
        FinalityNotificatorImpl::new(client.clone()),
    );
    let session_authorities = map_updater.readonly_session_map();
    spawn_handle.spawn("aleph/updater", None, async move {
        debug!(target: "aleph-party", "SessionMapUpdater has started.");
        map_updater.run(session_period).await
    });
    let (_, handler_task) = setup_justification_handler(JustificationParams {
        justification_rx,
        network,
        client,
        metrics,
        session_period,
        millisecs_per_block,
        session_map: session_authorities,
    });

    debug!(target: "aleph-party", "JustificationHandler has started.");
    handler_task.await;
    error!(target: "aleph-party", "JustificationHandler finished.");
}
