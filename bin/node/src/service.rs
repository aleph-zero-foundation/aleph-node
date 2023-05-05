//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use aleph_primitives::{AlephSessionApi, MAX_BLOCK_SIZE};
use aleph_runtime::{self, opaque::Block, RuntimeApi};
use finality_aleph::{
    run_validator_node, AlephBlockImport, AlephConfig, Justification, Metrics, MillisecsPerBlock,
    Protocol, ProtocolNaming, SessionPeriod, SubstrateChainStatus, TracingBlockImport,
};
use futures::channel::mpsc;
use log::{info, warn};
use sc_client_api::{BlockBackend, HeaderBackend};
use sc_consensus_aura::{ImportQueueParams, SlotProportion, StartAuraParams};
use sc_consensus_slots::BackoffAuthoringBlocksStrategy;
use sc_network::NetworkService;
use sc_service::{
    error::Error as ServiceError, Configuration, KeystoreContainer, NetworkStarter, RpcHandlers,
    TFullClient, TaskManager,
};
use sc_telemetry::{Telemetry, TelemetryWorker};
use sp_api::ProvideRuntimeApi;
use sp_arithmetic::traits::BaseArithmetic;
use sp_consensus_aura::{sr25519::AuthorityPair as AuraPair, Slot};
use sp_runtime::{
    generic::BlockId,
    traits::{Block as BlockT, Header as HeaderT},
};

use crate::{
    aleph_cli::AlephCli,
    chain_spec::DEFAULT_BACKUP_FOLDER,
    executor::AlephExecutor,
    rpc::{create_full as create_full_rpc, FullDeps as RpcFullDeps},
};

type FullClient = sc_service::TFullClient<Block, RuntimeApi, AlephExecutor>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;

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
        eprintln!("No backup path provided, using default path: {:?} for AlephBFT backups. Please do not remove this folder", path);
        Some(path)
    }
}

#[allow(clippy::type_complexity)]
pub fn new_partial(
    config: &Configuration,
) -> Result<
    sc_service::PartialComponents<
        FullClient,
        FullBackend,
        FullSelectChain,
        sc_consensus::DefaultImportQueue<Block, FullClient>,
        sc_transaction_pool::FullPool<Block, FullClient>,
        (
            TracingBlockImport<Block, Arc<FullClient>>,
            mpsc::UnboundedSender<Justification<<Block as BlockT>::Header>>,
            mpsc::UnboundedReceiver<Justification<<Block as BlockT>::Header>>,
            Option<Telemetry>,
            Metrics<<<Block as BlockT>::Header as HeaderT>::Hash>,
        ),
    >,
    ServiceError,
> {
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

    let executor = AlephExecutor::new(
        config.wasm_method,
        config.default_heap_pages,
        config.max_runtime_instances,
        config.runtime_cache_size,
    );

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

    let metrics = match config.prometheus_registry() {
        Some(register) => match Metrics::new(register) {
            Ok(metrics) => metrics,
            Err(e) => {
                warn!("Failed to register Prometheus metrics: {:?}.", e);
                Metrics::noop()
            }
        },
        None => {
            info!("Running with the metrics is not available.");
            Metrics::noop()
        }
    };

    let (justification_tx, justification_rx) = mpsc::unbounded();
    let tracing_block_import = TracingBlockImport::new(client.clone(), metrics.clone());
    let justification_translator = SubstrateChainStatus::new(backend.clone())
        .map_err(|e| ServiceError::Other(format!("failed to set up chain status: {}", e)))?;
    let aleph_block_import = AlephBlockImport::new(
        tracing_block_import.clone(),
        justification_tx.clone(),
        justification_translator,
    );

    let slot_duration = sc_consensus_aura::slot_duration(&*client)?;

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
        other: (
            tracing_block_import,
            justification_tx,
            justification_rx,
            telemetry,
            metrics,
        ),
    })
}

#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
fn setup(
    mut config: Configuration,
    backend: Arc<FullBackend>,
    chain_status: SubstrateChainStatus<Block>,
    keystore_container: &KeystoreContainer,
    import_queue: sc_consensus::DefaultImportQueue<Block, FullClient>,
    transaction_pool: Arc<sc_transaction_pool::FullPool<Block, FullClient>>,
    task_manager: &mut TaskManager,
    client: Arc<FullClient>,
    telemetry: &mut Option<Telemetry>,
    import_justification_tx: mpsc::UnboundedSender<Justification<<Block as BlockT>::Header>>,
) -> Result<
    (
        RpcHandlers,
        Arc<NetworkService<Block, <Block as BlockT>::Hash>>,
        ProtocolNaming,
        NetworkStarter,
    ),
    ServiceError,
> {
    let genesis_hash = client
        .block_hash(0)
        .ok()
        .flatten()
        .expect("we should have a hash");
    let chain_prefix = match config.chain_spec.fork_id() {
        Some(fork_id) => format!("/{}/{}", genesis_hash, fork_id),
        None => format!("/{}", genesis_hash),
    };
    let protocol_naming = ProtocolNaming::new(chain_prefix);
    config
        .network
        .extra_sets
        .push(finality_aleph::peers_set_config(
            protocol_naming.clone(),
            Protocol::Authentication,
        ));
    config
        .network
        .extra_sets
        .push(finality_aleph::peers_set_config(
            protocol_naming.clone(),
            Protocol::BlockSync,
        ));

    let (network, system_rpc_tx, tx_handler_controller, network_starter) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            block_announce_validator_builder: None,
            warp_sync: None,
        })?;

    let rpc_builder = {
        let client = client.clone();
        let pool = transaction_pool.clone();
        Box::new(move |deny_unsafe, _| {
            let deps = RpcFullDeps {
                client: client.clone(),
                pool: pool.clone(),
                deny_unsafe,
                import_justification_tx: import_justification_tx.clone(),
                justification_translator: chain_status.clone(),
            };

            Ok(create_full_rpc(deps)?)
        })
    };

    let rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        network: network.clone(),
        client,
        keystore: keystore_container.sync_keystore(),
        task_manager,
        transaction_pool,
        rpc_builder,
        backend,
        system_rpc_tx,
        tx_handler_controller,
        config,
        telemetry: telemetry.as_mut(),
    })?;

    Ok((rpc_handlers, network, protocol_naming, network_starter))
}

/// Builds a new service for a full client.
pub fn new_authority(
    config: Configuration,
    aleph_config: AlephCli,
) -> Result<TaskManager, ServiceError> {
    let sc_service::PartialComponents {
        client,
        backend,
        mut task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool,
        other: (block_import, justification_tx, justification_rx, mut telemetry, metrics),
    } = new_partial(&config)?;

    let backup_path = backup_path(
        &aleph_config,
        config
            .base_path
            .as_ref()
            .expect("Please specify base path")
            .path(),
    );

    let finalized = client.info().finalized_number;

    let session_period = SessionPeriod(
        client
            .runtime_api()
            .session_period(&BlockId::Number(finalized))
            .unwrap(),
    );

    let millisecs_per_block = MillisecsPerBlock(
        client
            .runtime_api()
            .millisecs_per_block(&BlockId::Number(finalized))
            .unwrap(),
    );

    let force_authoring = config.force_authoring;
    let backoff_authoring_blocks = Some(LimitNonfinalized(aleph_config.max_nonfinalized_blocks()));
    let prometheus_registry = config.prometheus_registry().cloned();

    let chain_status = SubstrateChainStatus::new(backend.clone())
        .map_err(|e| ServiceError::Other(format!("failed to set up chain status: {}", e)))?;
    let (_rpc_handlers, network, protocol_naming, network_starter) = setup(
        config,
        backend,
        chain_status.clone(),
        &keystore_container,
        import_queue,
        transaction_pool.clone(),
        &mut task_manager,
        client.clone(),
        &mut telemetry,
        justification_tx,
    )?;

    let mut proposer_factory = sc_basic_authorship::ProposerFactory::new(
        task_manager.spawn_handle(),
        client.clone(),
        transaction_pool,
        prometheus_registry.as_ref(),
        None,
    );
    proposer_factory.set_default_block_size_limit(MAX_BLOCK_SIZE as usize);

    let slot_duration = sc_consensus_aura::slot_duration(&*client)?;

    let aura = sc_consensus_aura::start_aura::<AuraPair, _, _, _, _, _, _, _, _, _, _>(
        StartAuraParams {
            slot_duration,
            client: client.clone(),
            select_chain: select_chain.clone(),
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
            force_authoring,
            backoff_authoring_blocks,
            keystore: keystore_container.sync_keystore(),
            sync_oracle: network.clone(),
            justification_sync_link: network.clone(),
            block_proposal_slot_portion: SlotProportion::new(2f32 / 3f32),
            max_block_proposal_slot_portion: None,
            telemetry: telemetry.as_ref().map(|x| x.handle()),
            compatibility_mode: Default::default(),
        },
    )?;

    task_manager
        .spawn_essential_handle()
        .spawn_blocking("aura", None, aura);

    if aleph_config.external_addresses().is_empty() {
        panic!("Cannot run a validator node without external addresses, stopping.");
    }
    let aleph_config = AlephConfig {
        network,
        client,
        chain_status,
        select_chain,
        session_period,
        millisecs_per_block,
        spawn_handle: task_manager.spawn_handle().into(),
        keystore: keystore_container.keystore(),
        justification_rx,
        metrics,
        unit_creation_delay: aleph_config.unit_creation_delay(),
        backup_saving_path: backup_path,
        external_addresses: aleph_config.external_addresses(),
        validator_port: aleph_config.validator_port(),
        protocol_naming,
    };

    task_manager.spawn_essential_handle().spawn_blocking(
        "aleph",
        None,
        run_validator_node(aleph_config),
    );

    network_starter.start_network();
    Ok(task_manager)
}
