//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use aleph_runtime::{self, opaque::Block, RuntimeApi};
use finality_aleph::{
    run_validator_node, AlephBlockImport, AlephConfig, AllBlockMetrics, BlockImporter,
    ChannelProvider, Justification, JustificationTranslator, MillisecsPerBlock, Protocol,
    ProtocolNaming, RateLimiterConfig, RedirectingBlockImport, SessionPeriod, SubstrateChainStatus,
    SubstrateNetwork, SyncOracle, TracingBlockImport, ValidatorAddressCache,
};
use log::warn;
use sc_basic_authorship::ProposerFactory;
use sc_client_api::{BlockBackend, HeaderBackend};
use sc_consensus::ImportQueue;
use sc_consensus_aura::{ImportQueueParams, SlotProportion, StartAuraParams};
use sc_consensus_slots::BackoffAuthoringBlocksStrategy;
use sc_network::config::FullNetworkConfiguration;
use sc_service::{error::Error as ServiceError, Configuration, TFullClient, TaskManager};
use sc_telemetry::{Telemetry, TelemetryWorker};
use sp_api::ProvideRuntimeApi;
use sp_arithmetic::traits::BaseArithmetic;
use sp_consensus::DisableProofRecording;
use sp_consensus_aura::{sr25519::AuthorityPair as AuraPair, Slot};

use crate::{
    aleph_cli::AlephCli,
    aleph_primitives::{AlephSessionApi, MAX_BLOCK_SIZE},
    chain_spec::DEFAULT_BACKUP_FOLDER,
    executor::AlephExecutor,
    rpc::{create_full as create_full_rpc, FullDeps as RpcFullDeps},
};

type FullClient = sc_service::TFullClient<Block, RuntimeApi, AlephExecutor>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
type FullPool = sc_transaction_pool::FullPool<Block, FullClient>;
type FullImportQueue = sc_consensus::DefaultImportQueue<Block>;
type FullProposerFactory =
    ProposerFactory<FullPool, FullBackend, FullClient, DisableProofRecording>;
type ServiceComponents = sc_service::PartialComponents<
    FullClient,
    FullBackend,
    FullSelectChain,
    FullImportQueue,
    FullPool,
    (
        ChannelProvider<Justification>,
        Option<Telemetry>,
        AllBlockMetrics,
    ),
>;

struct LimitNonfinalized(u32);

impl<N: BaseArithmetic> BackoffAuthoringBlocksStrategy<N> for LimitNonfinalized {
    fn should_backoff(
        &self,
        chain_head_number: N,
        _chain_head_slot: Slot,
        finalized_number: N,
        _slow_now: Slot,
        _logging_target: &str,
    ) -> bool {
        let nonfinalized_blocks: u32 = chain_head_number
            .saturating_sub(finalized_number)
            .unique_saturated_into();
        match nonfinalized_blocks >= self.0 {
            true => {
                warn!("We have {} nonfinalized blocks, with the limit being {}, delaying block production.", nonfinalized_blocks, self.0);
                true
            }
            false => false,
        }
    }
}

fn backup_path(aleph_config: &AlephCli, base_path: &Path) -> Option<PathBuf> {
    if aleph_config.no_backup() {
        return None;
    }
    if let Some(path) = aleph_config.backup_path() {
        Some(path)
    } else {
        let path = base_path.join(DEFAULT_BACKUP_FOLDER);
        eprintln!("No backup path provided, using default path: {path:?} for AlephBFT backups. Please do not remove this folder");
        Some(path)
    }
}

pub fn new_partial(config: &Configuration) -> Result<ServiceComponents, ServiceError> {
    let telemetry = config
        .telemetry_endpoints
        .clone()
        .filter(|x| !x.is_empty())
        .map(|endpoints| -> Result<_, sc_telemetry::Error> {
            let worker = TelemetryWorker::new(16)?;
            let telemetry = worker.handle().new_telemetry(endpoints);
            Ok((worker, telemetry))
        })
        .transpose()?;

    let executor = sc_service::new_native_or_wasm_executor(config);

    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts::<Block, RuntimeApi, AlephExecutor>(
            config,
            telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
            executor,
        )?;

    let telemetry = telemetry.map(|(worker, telemetry)| {
        task_manager
            .spawn_handle()
            .spawn("telemetry", None, worker.run());
        telemetry
    });

    let client: Arc<TFullClient<_, _, _>> = Arc::new(client);

    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        config.role.is_authority().into(),
        config.prometheus_registry(),
        task_manager.spawn_essential_handle(),
        client.clone(),
    );

    let metrics = AllBlockMetrics::new(config.prometheus_registry());

    let justification_channel_provider = ChannelProvider::new();
    let tracing_block_import = TracingBlockImport::new(client.clone(), metrics.clone());
    let justification_translator = JustificationTranslator::new(
        SubstrateChainStatus::new(backend.clone())
            .map_err(|e| ServiceError::Other(format!("failed to set up chain status: {e}")))?,
    );
    let aleph_block_import = AlephBlockImport::new(
        tracing_block_import,
        justification_channel_provider.get_sender(),
        justification_translator,
    );

    let slot_duration = sc_consensus_aura::slot_duration(&*client)?;

    // DO NOT change Aura parameters without updating the finality-aleph sync accordingly,
    // in particular the code responsible for verifying incoming Headers, as it is supposed
    // to duplicate parts of Aura internal logic
    let import_queue = sc_consensus_aura::import_queue::<AuraPair, _, _, _, _, _>(
        ImportQueueParams {
            block_import: aleph_block_import.clone(),
            justification_import: Some(Box::new(aleph_block_import)),
            client: client.clone(),
            create_inherent_data_providers: move |_, ()| async move {
                let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

                let slot =
                    sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                        *timestamp,
                        slot_duration,
                    );

                Ok((slot, timestamp))
            },
            spawner: &task_manager.spawn_essential_handle(),
            registry: config.prometheus_registry(),
            check_for_equivocation: Default::default(),
            telemetry: telemetry.as_ref().map(|x| x.handle()),
            compatibility_mode: Default::default(),
        },
    )?;

    Ok(sc_service::PartialComponents {
        client,
        backend,
        task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool,
        other: (justification_channel_provider, telemetry, metrics),
    })
}

struct AlephRuntimeVars {
    pub session_period: SessionPeriod,
    pub millisecs_per_block: MillisecsPerBlock,
}

fn get_aleph_runtime_vars(client: &Arc<FullClient>) -> AlephRuntimeVars {
    let finalized = client.info().finalized_hash;

    let session_period = SessionPeriod(
        client
            .runtime_api()
            .session_period(finalized)
            .expect("should always be available"),
    );

    let millisecs_per_block = MillisecsPerBlock(
        client
            .runtime_api()
            .millisecs_per_block(finalized)
            .expect("should always be available"),
    );

    AlephRuntimeVars {
        session_period,
        millisecs_per_block,
    }
}

fn get_validator_address_cache(aleph_config: &AlephCli) -> Option<ValidatorAddressCache> {
    aleph_config
        .no_collection_of_extra_debugging_data()
        .then(ValidatorAddressCache::new)
}

fn get_net_config(
    config: &Configuration,
    client: &Arc<FullClient>,
) -> (FullNetworkConfiguration, ProtocolNaming) {
    let genesis_hash = client
        .block_hash(0)
        .ok()
        .flatten()
        .expect("we should have a hash");
    let chain_prefix = match config.chain_spec.fork_id() {
        Some(fork_id) => format!("/{genesis_hash}/{fork_id}"),
        None => format!("/{genesis_hash}"),
    };
    let protocol_naming = ProtocolNaming::new(chain_prefix);
    let mut net_config = FullNetworkConfiguration::new(&config.network);
    net_config.add_notification_protocol(finality_aleph::peers_set_config(
        protocol_naming.clone(),
        Protocol::Authentication,
    ));
    net_config.add_notification_protocol(finality_aleph::peers_set_config(
        protocol_naming.clone(),
        Protocol::BlockSync,
    ));

    (net_config, protocol_naming)
}

fn get_proposer_factory(
    service_components: &ServiceComponents,
    config: &Configuration,
) -> FullProposerFactory {
    let mut proposer_factory = FullProposerFactory::new(
        service_components.task_manager.spawn_handle(),
        service_components.client.clone(),
        service_components.transaction_pool.clone(),
        config.prometheus_registry().cloned().as_ref(),
        None,
    );
    proposer_factory.set_default_block_size_limit(MAX_BLOCK_SIZE as usize);

    proposer_factory
}

fn get_rate_limit_config(aleph_config: &AlephCli) -> RateLimiterConfig {
    RateLimiterConfig {
        alephbft_bit_rate_per_connection: aleph_config
            .alephbft_bit_rate_per_connection()
            .try_into()
            .unwrap_or(usize::MAX),
    }
}

/// Builds a new service for a full client.
pub fn new_authority(
    config: Configuration,
    aleph_config: AlephCli,
) -> Result<TaskManager, ServiceError> {
    if aleph_config.external_addresses().is_empty() {
        panic!("Cannot run a validator node without external addresses, stopping.");
    }

    let mut service_components = new_partial(&config)?;

    let backup_path = backup_path(&aleph_config, config.base_path.path());

    let backoff_authoring_blocks = Some(LimitNonfinalized(aleph_config.max_nonfinalized_blocks()));
    let prometheus_registry = config.prometheus_registry().cloned();
    let (sync_oracle, _) = SyncOracle::new();
    let proposer_factory = get_proposer_factory(&service_components, &config);
    let slot_duration = sc_consensus_aura::slot_duration(&*service_components.client)?;
    let (block_import, block_rx) = RedirectingBlockImport::new(service_components.client.clone());

    let aura = sc_consensus_aura::start_aura::<AuraPair, _, _, _, _, _, _, _, _, _, _>(
        StartAuraParams {
            slot_duration,
            client: service_components.client.clone(),
            select_chain: service_components.select_chain.clone(),
            block_import,
            proposer_factory,
            create_inherent_data_providers: move |_, ()| async move {
                let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

                let slot =
                    sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                        *timestamp,
                        slot_duration,
                    );

                Ok((slot, timestamp))
            },
            force_authoring: config.force_authoring,
            backoff_authoring_blocks,
            keystore: service_components.keystore_container.local_keystore(),
            sync_oracle: sync_oracle.clone(),
            justification_sync_link: (),
            block_proposal_slot_portion: SlotProportion::new(2f32 / 3f32),
            max_block_proposal_slot_portion: None,
            telemetry: service_components.other.1.as_ref().map(|x| x.handle()),
            compatibility_mode: Default::default(),
        },
    )?;

    let import_queue_handle = BlockImporter::new(service_components.import_queue.service());
    let (net_config, protocol_naming) = get_net_config(&config, &service_components.client);
    let (network, system_rpc_tx, tx_handler_controller, network_starter, sync_network) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            net_config,
            client: service_components.client.clone(),
            transaction_pool: service_components.transaction_pool.clone(),
            spawn_handle: service_components.task_manager.spawn_handle(),
            import_queue: service_components.import_queue,
            block_announce_validator_builder: None,
            warp_sync_params: None,
            block_relay: None,
        })?;

    let chain_status = SubstrateChainStatus::new(service_components.backend.clone())
        .map_err(|e| ServiceError::Other(format!("failed to set up chain status: {e}")))?;
    let validator_address_cache = get_validator_address_cache(&aleph_config);
    let rpc_builder = {
        let client = service_components.client.clone();
        let pool = service_components.transaction_pool.clone();
        let sync_oracle = sync_oracle.clone();
        let validator_address_cache = validator_address_cache.clone();
        let import_justification_tx = service_components.other.0.get_sender();
        let chain_status = chain_status.clone();
        Box::new(move |deny_unsafe, _| {
            let deps = RpcFullDeps {
                client: client.clone(),
                pool: pool.clone(),
                deny_unsafe,
                import_justification_tx: import_justification_tx.clone(),
                justification_translator: JustificationTranslator::new(chain_status.clone()),
                sync_oracle: sync_oracle.clone(),
                validator_address_cache: validator_address_cache.clone(),
            };

            Ok(create_full_rpc(deps)?)
        })
    };

    let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        network: network.clone(),
        sync_service: sync_network.clone(),
        client: service_components.client.clone(),
        keystore: service_components.keystore_container.local_keystore(),
        task_manager: &mut service_components.task_manager,
        transaction_pool: service_components.transaction_pool.clone(),
        rpc_builder,
        backend: service_components.backend,
        system_rpc_tx,
        tx_handler_controller,
        config,
        telemetry: service_components.other.1.as_mut(),
    })?;

    service_components
        .task_manager
        .spawn_essential_handle()
        .spawn_blocking("aura", None, aura);

    let rate_limiter_config = get_rate_limit_config(&aleph_config);

    let substrate_network = SubstrateNetwork::new(network, sync_network, protocol_naming);
    // Network event stream needs to be created before starting the network,
    // otherwise some events might be missed.
    let network_event_stream = substrate_network.event_stream();

    let AlephRuntimeVars {
        millisecs_per_block,
        session_period,
    } = get_aleph_runtime_vars(&service_components.client);

    let aleph_config = AlephConfig {
        network: substrate_network,
        network_event_stream,
        client: service_components.client,
        chain_status,
        import_queue_handle,
        select_chain: service_components.select_chain,
        session_period,
        millisecs_per_block,
        spawn_handle: service_components.task_manager.spawn_handle().into(),
        keystore: service_components.keystore_container.local_keystore(),
        justification_channel_provider: service_components.other.0,
        block_rx,
        metrics: service_components.other.2,
        registry: prometheus_registry,
        unit_creation_delay: aleph_config.unit_creation_delay(),
        backup_saving_path: backup_path,
        external_addresses: aleph_config.external_addresses(),
        validator_port: aleph_config.validator_port(),
        rate_limiter_config,
        sync_oracle,
        validator_address_cache,
        transaction_pool: service_components.transaction_pool,
    };

    service_components
        .task_manager
        .spawn_essential_handle()
        .spawn_blocking("aleph", None, run_validator_node(aleph_config));

    network_starter.start_network();
    Ok(service_components.task_manager)
}
