use std::{marker::PhantomData, sync::Arc};

use bip39::{Language, Mnemonic, MnemonicType};
use futures::channel::oneshot;
use log::{debug, error};
use sc_client_api::Backend;
use sc_network_common::ExHashT;
use sp_consensus::SelectChain;
use sp_keystore::CryptoStore;
use sp_runtime::traits::Block;

use crate::{
    crypto::AuthorityPen,
    network::{
        clique::Service,
        session::{ConnectionManager, ConnectionManagerConfig},
        tcp::{new_tcp_network, KEY_TYPE},
        GossipService, SubstrateNetwork,
    },
    nodes::{setup_justification_handler, JustificationParams},
    party::{
        impls::{ChainStateImpl, SessionInfoImpl},
        manager::NodeSessionManagerImpl,
        ConsensusParty, ConsensusPartyParams,
    },
    session_map::{AuthorityProviderImpl, FinalityNotificatorImpl, SessionMapUpdater},
    AlephConfig, BlockchainBackend,
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

pub async fn run_validator_node<B, H, C, BB, BE, SC>(aleph_config: AlephConfig<B, H, C, SC, BB>)
where
    B: Block,
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
        protocol_naming,
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
        &network_authority_pen,
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

    let (gossip_network_service, authentication_network, _block_sync_network) = GossipService::new(
        SubstrateNetwork::new(network.clone(), protocol_naming),
        spawn_handle.clone(),
    );
    let gossip_network_task = async move { gossip_network_service.run().await };

    let block_requester = network.clone();
    let map_updater = SessionMapUpdater::<_, _, B>::new(
        AuthorityProviderImpl::new(client.clone()),
        FinalityNotificatorImpl::new(client.clone()),
        session_period,
    );
    let session_authorities = map_updater.readonly_session_map();
    spawn_handle.spawn("aleph/updater", None, async move {
        debug!(target: "aleph-party", "SessionMapUpdater has started.");
        map_updater.run().await
    });

    let (authority_justification_tx, handler_task) =
        setup_justification_handler(JustificationParams {
            justification_rx,
            network,
            client: client.clone(),
            blockchain_backend,
            metrics: metrics.clone(),
            session_period,
            millisecs_per_block,
            session_map: session_authorities.clone(),
        });

    let (connection_manager_service, connection_manager) = ConnectionManager::new(
        network_identity,
        validator_network,
        authentication_network,
        ConnectionManagerConfig::with_session_period(&session_period, &millisecs_per_block),
    );

    let connection_manager_task = async move {
        if let Err(e) = connection_manager_service.run().await {
            panic!("Failed to run connection manager: {}", e);
        }
    };

    spawn_handle.spawn("aleph/justification_handler", None, handler_task);
    debug!(target: "aleph-party", "JustificationHandler has started.");

    spawn_handle.spawn("aleph/connection_manager", None, connection_manager_task);
    spawn_handle.spawn("aleph/gossip_network", None, gossip_network_task);
    debug!(target: "aleph-party", "Gossip network has started.");

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
            connection_manager,
            keystore,
        ),
        _phantom: PhantomData,
        session_info: SessionInfoImpl::new(session_period),
    });

    debug!(target: "aleph-party", "Consensus party has started.");
    party.run().await;
    error!(target: "aleph-party", "Consensus party has finished unexpectedly.");
}
