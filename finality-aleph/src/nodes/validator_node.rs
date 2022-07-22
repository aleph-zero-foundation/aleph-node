use std::marker::PhantomData;

use log::{debug, error};
use sc_client_api::Backend;
use sc_network::ExHashT;
use sp_consensus::SelectChain;
use sp_runtime::traits::Block;

use crate::{
    mpsc,
    network::{
        ConnectionIO, ConnectionManager, ConnectionManagerConfig, Service as NetworkService,
        SessionManager, IO as NetworkIO,
    },
    nodes::{setup_justification_handler, JustificationParams},
    party::{
        impls::{ChainStateImpl, SessionInfoImpl},
        manager::NodeSessionManagerImpl,
        ConsensusParty, ConsensusPartyParams,
    },
    session_map::{AuthorityProviderImpl, FinalityNotificatorImpl, SessionMapUpdater},
    AlephConfig,
};

pub async fn run_validator_node<B, H, C, BE, SC>(aleph_config: AlephConfig<B, H, C, SC>)
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
        select_chain,
        spawn_handle,
        keystore,
        metrics,
        unit_creation_delay,
        session_period,
        millisecs_per_block,
        justification_rx,
        backup_saving_path,
        ..
    } = aleph_config;

    let block_requester = network.clone();
    let map_updater = SessionMapUpdater::<_, _, B>::new(
        AuthorityProviderImpl::new(client.clone()),
        FinalityNotificatorImpl::new(client.clone()),
    );
    let session_authorities = map_updater.readonly_session_map();
    spawn_handle.spawn("aleph/updater", None, async move {
        debug!(target: "aleph-party", "SessionMapUpdater has started.");
        map_updater.run(session_period).await
    });

    let (authority_justification_tx, handler_task) =
        setup_justification_handler(JustificationParams {
            justification_rx,
            network: network.clone(),
            client: client.clone(),
            metrics: metrics.clone(),
            session_period,
            millisecs_per_block,
            session_map: session_authorities.clone(),
        });

    // Prepare and start the network
    let (commands_for_network, commands_from_io) = mpsc::unbounded();
    let (messages_for_network, messages_from_user) = mpsc::unbounded();
    let (commands_for_service, commands_from_user) = mpsc::unbounded();
    let (messages_for_service, commands_from_manager) = mpsc::unbounded();
    let (messages_for_user, messages_from_network) = mpsc::unbounded();

    let connection_io = ConnectionIO::new(
        commands_for_network,
        messages_for_network,
        commands_from_user,
        commands_from_manager,
        messages_from_network,
    );
    let connection_manager = ConnectionManager::new(
        network.clone(),
        ConnectionManagerConfig::with_session_period(&session_period, &millisecs_per_block),
    );
    let session_manager = SessionManager::new(commands_for_service, messages_for_service);
    let network = NetworkService::new(
        network.clone(),
        spawn_handle.clone(),
        NetworkIO::new(messages_from_user, messages_for_user, commands_from_io),
    );

    let network_manager_task = async move {
        connection_io
            .run(connection_manager)
            .await
            .expect("Failed to run new network manager")
    };

    let network_task = async move { network.run().await };

    spawn_handle.spawn("aleph/justification_handler", None, handler_task);
    debug!(target: "aleph-party", "JustificationHandler has started.");

    spawn_handle.spawn("aleph/network_manager", None, network_manager_task);
    spawn_handle.spawn("aleph/network", None, network_task);
    debug!(target: "aleph-party", "Network has started.");

    let party = ConsensusParty::new(ConsensusPartyParams {
        session_authorities,
        sync_state: block_requester.clone(),
        backup_saving_path,
        chain_state: ChainStateImpl {
            client: client.clone(),
            _phantom: PhantomData,
        },
        session_manager: NodeSessionManagerImpl::new(
            client,
            select_chain,
            session_period,
            unit_creation_delay,
            authority_justification_tx,
            block_requester,
            metrics,
            spawn_handle.into(),
            session_manager,
            keystore,
        ),
        _phantom: PhantomData,
        session_info: SessionInfoImpl::new(session_period),
    });

    debug!(target: "aleph-party", "Consensus party has started.");
    party.run().await;
    error!(target: "aleph-party", "Consensus party has finished unexpectedly.");
}
