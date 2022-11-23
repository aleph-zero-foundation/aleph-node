use std::{marker::PhantomData, sync::Arc};

use bip39::{Language, Mnemonic, MnemonicType};
use futures::channel::oneshot;
use log::{debug, error};
use sc_client_api::Backend;
use sc_network::ExHashT;
use sp_consensus::SelectChain;
use sp_keystore::CryptoStore;
use sp_runtime::traits::Block;

use crate::{
    crypto::AuthorityPen,
    network::{
        setup_io, ConnectionManager, ConnectionManagerConfig, Service as NetworkService,
        SessionManager,
    },
    nodes::{setup_justification_handler, JustificationParams},
    party::{
        impls::{ChainStateImpl, SessionInfoImpl},
        manager::NodeSessionManagerImpl,
        ConsensusParty, ConsensusPartyParams,
    },
    session_map::{AuthorityProviderImpl, FinalityNotificatorImpl, SessionMapUpdater},
    tcp_network::new_tcp_network,
    validator_network::{Service, KEY_TYPE},
    AlephConfig,
};

pub async fn new_pen(mnemonic: &str, keystore: Arc<dyn CryptoStore>) -> AuthorityPen {
    let validator_peer_id = keystore
        .ed25519_generate_new(KEY_TYPE, Some(mnemonic))
        .await
        .expect("generating a key should work");
    AuthorityPen::new_with_key_type(validator_peer_id.into(), keystore, KEY_TYPE)
        .await
        .expect("we just generated this key so everything should work")
}

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
        external_addresses,
        validator_port,
        ..
    } = aleph_config;

    // We generate the phrase manually to only save the key in RAM, we don't want to have these
    // relatively low-importance keys getting spammed around the absolutely crucial Aleph keys.
    // The interface of `ed25519_generate_new` only allows to save in RAM by providing a mnemonic.
    let network_authority_pen = new_pen(
        Mnemonic::new(MnemonicType::Words12, Language::English).phrase(),
        keystore.clone(),
    )
    .await;
    let (dialer, listener, network_identity) = new_tcp_network(
        ("0.0.0.0", validator_port),
        external_addresses,
        network_authority_pen.authority_id(),
    )
    .await
    .expect("we should have working networking");
    let (validator_network_service, validator_network) = Service::new(
        dialer,
        listener,
        network_authority_pen,
        spawn_handle.clone(),
    );
    let (_validator_network_exit, exit) = oneshot::channel();
    spawn_handle.spawn("aleph/validator_network", None, async move {
        debug!(target: "aleph-party", "Validator network has started.");
        validator_network_service.run(exit).await
    });

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

    let (connection_io, network_io, session_io) = setup_io();

    let connection_manager = ConnectionManager::new(
        network_identity,
        ConnectionManagerConfig::with_session_period(&session_period, &millisecs_per_block),
    );

    let connection_manager_task = async move {
        connection_io
            .run(connection_manager)
            .await
            .expect("Failed to run connection manager")
    };

    let session_manager = SessionManager::new(session_io);
    let network = NetworkService::new(
        network.clone(),
        validator_network,
        spawn_handle.clone(),
        network_io,
    );
    let network_task = async move { network.run().await };

    spawn_handle.spawn("aleph/justification_handler", None, handler_task);
    debug!(target: "aleph-party", "JustificationHandler has started.");

    spawn_handle.spawn("aleph/connection_manager", None, connection_manager_task);
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
