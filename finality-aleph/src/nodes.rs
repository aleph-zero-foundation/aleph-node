use std::{marker::PhantomData, sync::Arc};

use bip39::{Language, Mnemonic, MnemonicType};
use futures::channel::oneshot;
use log::{debug, error};
use network_clique::{RateLimitingDialer, RateLimitingListener, Service, SpawnHandleT};
use pallet_aleph_runtime_api::AlephSessionApi;
use primitives::TransactionHash;
use rate_limiter::SleepingRateLimiter;
use sc_client_api::Backend;
use sc_keystore::{Keystore, LocalKeystore};
use sc_transaction_pool_api::TransactionPool;
use sp_consensus_aura::AuraApi;

use crate::{
    aleph_primitives::{AuraId, Block},
    block::{
        substrate::{JustificationTranslator, SubstrateFinalizationInfo, VerifierCache},
        BlockchainEvents, ChainStatus, FinalizationStatus, Justification,
    },
    crypto::AuthorityPen,
    finalization::AlephFinalizer,
    idx_to_account::ValidatorIndexToAccountIdConverterImpl,
    metrics::{run_metrics_service, SloMetrics},
    network::{
        address_cache::validator_address_cache_updater,
        session::{ConnectionManager, ConnectionManagerConfig},
        tcp::{new_tcp_network, KEY_TYPE},
    },
    party::{
        impls::ChainStateImpl, manager::NodeSessionManagerImpl, ConsensusParty,
        ConsensusPartyParams,
    },
    runtime_api::RuntimeApiImpl,
    session::SessionBoundaryInfo,
    session_map::{AuthorityProviderImpl, FinalityNotifierImpl, SessionMapUpdater},
    sync::{DatabaseIO as SyncDatabaseIO, Service as SyncService, IO as SyncIO},
    AlephConfig,
};

// How many sessions we remember.
// Keep in mind that Aura stores authority info in the parent block,
// so the actual size probably needs to be increased by one.
pub const VERIFIER_CACHE_SIZE: usize = 3;

const LOG_TARGET: &str = "aleph-party";

pub fn new_pen(mnemonic: &str, keystore: Arc<LocalKeystore>) -> AuthorityPen {
    let validator_peer_id = keystore
        .ed25519_generate_new(KEY_TYPE, Some(mnemonic))
        .expect("generating a key should work");
    AuthorityPen::new_with_key_type(validator_peer_id.into(), keystore, KEY_TYPE)
        .expect("we just generated this key so everything should work")
}

pub async fn run_validator_node<C, BE, TP>(aleph_config: AlephConfig<C, TP>)
where
    C: crate::ClientForAleph<Block, BE> + Send + Sync + 'static,
    C::Api: AlephSessionApi<Block> + AuraApi<Block, AuraId>,
    BE: Backend<Block> + 'static,
    TP: TransactionPool<Block = Block, Hash = TransactionHash> + 'static,
{
    let AlephConfig {
        authentication_network,
        block_sync_network,
        client,
        chain_status,
        mut import_queue_handle,
        select_chain_provider,
        spawn_handle,
        keystore,
        registry,
        unit_creation_delay,
        session_period,
        millisecs_per_block,
        justification_channel_provider,
        block_rx,
        backup_saving_path,
        external_addresses,
        validator_port,
        rate_limiter_config,
        sync_oracle,
        validator_address_cache,
        transaction_pool,
    } = aleph_config;

    // We generate the phrase manually to only save the key in RAM, we don't want to have these
    // relatively low-importance keys getting spammed around the absolutely crucial Aleph keys.
    // The interface of `ed25519_generate_new` only allows to save in RAM by providing a mnemonic.
    let network_authority_pen = new_pen(
        Mnemonic::new(MnemonicType::Words12, Language::English).phrase(),
        keystore.clone(),
    );

    debug!(
        target: LOG_TARGET,
        "Initializing rate-limiter for the validator-network with {} byte(s) per second.",
        rate_limiter_config.alephbft_bit_rate_per_connection
    );

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
        debug!(target: LOG_TARGET, "Validator network has started.");
        match validator_network_service.run(exit).await {
            Ok(_) => debug!(target: LOG_TARGET, "Validator network finished."),
            Err(err) => error!(
                target: LOG_TARGET,
                "Validator network finished with error: {err}."
            ),
        }
    });

    let map_updater = SessionMapUpdater::new(
        AuthorityProviderImpl::new(client.clone(), RuntimeApiImpl::new(client.clone())),
        FinalityNotifierImpl::new(client.clone()),
        session_period,
    );
    let session_authorities = map_updater.readonly_session_map();
    spawn_handle.spawn("aleph/updater", async move {
        debug!(target: LOG_TARGET, "SessionMapUpdater has started.");
        map_updater.run().await;
        debug!(target: LOG_TARGET, "SessionMapUpdater finished.");
    });

    let chain_events = client.chain_status_notifier();

    let slo_metrics = SloMetrics::new(registry.as_ref(), chain_status.clone());
    let timing_metrics = slo_metrics.timing_metrics().clone();

    spawn_handle.spawn("aleph/slo-metrics", {
        let slo_metrics = slo_metrics.clone();
        async move {
            run_metrics_service(
                &slo_metrics,
                &mut transaction_pool.import_notification_stream(),
            )
            .await;
        }
    });

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
        AuthorityProviderImpl::new(client.clone(), RuntimeApiImpl::new(client.clone())),
        VERIFIER_CACHE_SIZE,
        genesis_header,
    );
    let finalizer = AlephFinalizer::new(client.clone());
    import_queue_handle.attach_metrics(timing_metrics.clone());
    let justifications_for_sync = justification_channel_provider.get_sender();
    let sync_io = SyncIO::new(
        SyncDatabaseIO::new(chain_status.clone(), finalizer, import_queue_handle),
        block_sync_network,
        chain_events,
        sync_oracle.clone(),
        justification_channel_provider.into_receiver(),
        block_rx,
    );
    let select_chain = select_chain_provider.select_chain();
    let favourite_block_user_requests = select_chain_provider.favourite_block_user_requests();
    let (sync_service, request_block) = match SyncService::new(
        verifier.clone(),
        session_info.clone(),
        sync_io,
        registry.clone(),
        slo_metrics,
        favourite_block_user_requests,
    ) {
        Ok(x) => x,
        Err(e) => panic!("Failed to initialize Sync service: {e}"),
    };
    let sync_task = async move {
        if let Err(err) = sync_service.run().await {
            error!(
                target: LOG_TARGET,
                "Sync service finished with error: {err}."
            );
        }
    };

    let validator_address_cache_updater = validator_address_cache_updater(
        validator_address_cache,
        ValidatorIndexToAccountIdConverterImpl::new(
            client.clone(),
            session_info.clone(),
            RuntimeApiImpl::new(client.clone()),
        ),
    );

    let (connection_manager_service, connection_manager) = ConnectionManager::new(
        network_identity,
        validator_network,
        authentication_network,
        validator_address_cache_updater,
        ConnectionManagerConfig::with_session_period(&session_period, &millisecs_per_block),
    );

    let connection_manager_task = async move {
        if let Err(e) = connection_manager_service.run().await {
            panic!("Failed to run connection manager: {e}");
        }
    };

    spawn_handle.spawn("aleph/sync", sync_task);
    debug!(target: LOG_TARGET, "Sync has started.");

    spawn_handle.spawn("aleph/connection_manager", connection_manager_task);
    debug!(target: LOG_TARGET, "Sync network has started.");

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
            chain_status.clone(),
            select_chain,
            verifier,
            session_period,
            unit_creation_delay,
            justifications_for_sync,
            JustificationTranslator::new(chain_status.clone()),
            request_block,
            timing_metrics,
            spawn_handle,
            connection_manager,
            keystore,
        ),
        session_info,
    });

    debug!(target: LOG_TARGET, "Consensus party has started.");
    party.run().await;
    error!(
        target: LOG_TARGET,
        "Consensus party has finished unexpectedly."
    );
}
