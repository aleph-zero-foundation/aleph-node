use std::{marker::PhantomData, sync::Arc};

use bip39::{Language, Mnemonic, MnemonicType};
use futures::channel::oneshot;
use log::{debug, error};
use network_clique::{RateLimitingDialer, RateLimitingListener, Service, SpawnHandleT};
use rate_limiter::SleepingRateLimiter;
use sc_client_api::Backend;
use sp_consensus::SelectChain;
use sp_keystore::Keystore;

use crate::{
    aleph_primitives::Block,
    crypto::AuthorityPen,
    finalization::AlephFinalizer,
    network::{
        session::{ConnectionManager, ConnectionManagerConfig},
        tcp::{new_tcp_network, KEY_TYPE},
        GossipService, SubstrateNetwork,
    },
    party::{
        impls::ChainStateImpl, manager::NodeSessionManagerImpl, ConsensusParty,
        ConsensusPartyParams,
    },
    session::SessionBoundaryInfo,
    session_map::{AuthorityProviderImpl, FinalityNotifierImpl, SessionMapUpdater},
    sync::{
        ChainStatus, FinalizationStatus, Justification, JustificationTranslator,
        OldSyncCompatibleRequestBlocks, Service as SyncService, SubstrateChainStatusNotifier,
        SubstrateFinalizationInfo, VerifierCache, IO as SyncIO,
    },
    AlephConfig,
};

// How many sessions we remember.
pub const VERIFIER_CACHE_SIZE: usize = 2;

pub fn new_pen(mnemonic: &str, keystore: Arc<dyn Keystore>) -> AuthorityPen {
    let validator_peer_id = keystore
        .ed25519_generate_new(KEY_TYPE, Some(mnemonic))
        .expect("generating a key should work");
    AuthorityPen::new_with_key_type(validator_peer_id.into(), keystore, KEY_TYPE)
        .expect("we just generated this key so everything should work")
}

pub async fn run_validator_node<C, BE, SC>(aleph_config: AlephConfig<C, SC>)
where
    C: crate::ClientForAleph<Block, BE> + Send + Sync + 'static,
    C::Api: crate::aleph_primitives::AlephSessionApi<Block>,
    BE: Backend<Block> + 'static,
    SC: SelectChain<Block> + 'static,
{
    let AlephConfig {
        network,
        sync_network,
        client,
        chain_status,
        mut import_queue_handle,
        select_chain,
        spawn_handle,
        keystore,
        metrics,
        registry,
        unit_creation_delay,
        session_period,
        millisecs_per_block,
        justification_rx,
        backup_saving_path,
        external_addresses,
        validator_port,
        protocol_naming,
        rate_limiter_config,
        sync_oracle,
    } = aleph_config;

    // We generate the phrase manually to only save the key in RAM, we don't want to have these
    // relatively low-importance keys getting spammed around the absolutely crucial Aleph keys.
    // The interface of `ed25519_generate_new` only allows to save in RAM by providing a mnemonic.
    let network_authority_pen = new_pen(
        Mnemonic::new(MnemonicType::Words12, Language::English).phrase(),
        keystore.clone(),
    );

    debug!(target: "aleph-party", "Initializing rate-limiter for the validator-network with {} byte(s) per second.", rate_limiter_config.alephbft_bit_rate_per_connection);

    let (dialer, listener, network_identity) = new_tcp_network(
        ("0.0.0.0", validator_port),
        external_addresses,
        &network_authority_pen,
    )
    .await
    .expect("we should have working networking");

    let alephbft_rate_limiter =
        SleepingRateLimiter::new(rate_limiter_config.alephbft_bit_rate_per_connection);
    let dialer = RateLimitingDialer::new(dialer, alephbft_rate_limiter.clone());
    let listener = RateLimitingListener::new(listener, alephbft_rate_limiter);

    let (validator_network_service, validator_network) = Service::new(
        dialer,
        listener,
        network_authority_pen,
        spawn_handle.clone(),
        registry.clone(),
    );
    let (_validator_network_exit, exit) = oneshot::channel();
    spawn_handle.spawn("aleph/validator_network", async move {
        debug!(target: "aleph-party", "Validator network has started.");
        validator_network_service.run(exit).await
    });

    let (gossip_network_service, authentication_network, block_sync_network) = GossipService::new(
        SubstrateNetwork::new(network.clone(), sync_network.clone(), protocol_naming),
        spawn_handle.clone(),
        registry.clone(),
    );
    let gossip_network_task = async move { gossip_network_service.run().await };

    let block_requester = sync_network.clone();

    let map_updater = SessionMapUpdater::new(
        AuthorityProviderImpl::new(client.clone()),
        FinalityNotifierImpl::new(client.clone()),
        session_period,
    );
    let session_authorities = map_updater.readonly_session_map();
    spawn_handle.spawn("aleph/updater", async move {
        debug!(target: "aleph-party", "SessionMapUpdater has started.");
        map_updater.run().await
    });

    let chain_events = SubstrateChainStatusNotifier::new(
        client.finality_notification_stream(),
        client.every_import_notification_stream(),
    );

    let session_info = SessionBoundaryInfo::new(session_period);
    let genesis_header = match chain_status.finalized_at(0) {
        Ok(FinalizationStatus::FinalizedWithJustification(justification)) => {
            justification.header().clone()
        }
        _ => panic!("the genesis block should be finalized"),
    };
    let verifier = VerifierCache::new(
        session_info.clone(),
        SubstrateFinalizationInfo::new(client.clone()),
        AuthorityProviderImpl::new(client.clone()),
        VERIFIER_CACHE_SIZE,
        genesis_header,
    );
    let finalizer = AlephFinalizer::new(client.clone(), metrics.clone());
    import_queue_handle.attach_metrics(metrics.clone());
    let sync_io = SyncIO::new(
        chain_status.clone(),
        finalizer,
        import_queue_handle,
        block_sync_network,
        chain_events,
        sync_oracle.clone(),
        justification_rx,
    );
    let (sync_service, justifications_for_sync, request_block) =
        match SyncService::new(verifier, session_info.clone(), sync_io, registry.clone()) {
            Ok(x) => x,
            Err(e) => panic!("Failed to initialize Sync service: {e}"),
        };
    let sync_task = async move { sync_service.run().await };

    let (connection_manager_service, connection_manager) = ConnectionManager::new(
        network_identity,
        validator_network,
        authentication_network,
        ConnectionManagerConfig::with_session_period(&session_period, &millisecs_per_block),
    );

    let connection_manager_task = async move {
        if let Err(e) = connection_manager_service.run().await {
            panic!("Failed to run connection manager: {e}");
        }
    };

    spawn_handle.spawn("aleph/sync", sync_task);
    debug!(target: "aleph-party", "Sync has started.");

    spawn_handle.spawn("aleph/connection_manager", connection_manager_task);
    spawn_handle.spawn("aleph/gossip_network", gossip_network_task);
    debug!(target: "aleph-party", "Gossip network has started.");

    let compatible_block_request =
        OldSyncCompatibleRequestBlocks::new(block_requester.clone(), request_block);

    let party = ConsensusParty::new(ConsensusPartyParams {
        session_authorities,
        sync_oracle,
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
            justifications_for_sync,
            JustificationTranslator::new(chain_status.clone()),
            compatible_block_request,
            metrics,
            spawn_handle,
            connection_manager,
            keystore,
        ),
        session_info,
    });

    debug!(target: "aleph-party", "Consensus party has started.");
    party.run().await;
    error!(target: "aleph-party", "Consensus party has finished unexpectedly.");
}
