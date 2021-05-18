//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use aleph_runtime::{self, opaque::Block, RuntimeApi};
use codec::Decode;
use finality_aleph::{
    run_aleph_consensus, AlephBlockImport, AlephConfig, AuthorityId, AuthorityKeystore,
    ConsensusConfig,
};
use sc_client_api::{CallExecutor, ExecutionStrategy, ExecutorProvider};
use sc_consensus_aura::{ImportQueueParams, SlotProportion, StartAuraParams};
use sc_executor::native_executor_instance;
pub use sc_executor::NativeExecutor;
use sc_service::{error::Error as ServiceError, Configuration, TFullClient, TaskManager};
use sc_telemetry::TelemetryWorker;
use sp_consensus_aura::sr25519::AuthorityPair as AuraPair;
use sp_inherents::InherentDataProviders;
use sp_keystore::{SyncCryptoStore, SyncCryptoStorePtr};
use sp_runtime::{generic::BlockId, traits::Zero};
use std::sync::Arc;

// Our native executor instance.
native_executor_instance!(
    pub Executor,
    aleph_runtime::api::dispatch,
    aleph_runtime::native_version,
);

type FullClient = sc_service::TFullClient<Block, RuntimeApi, Executor>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;

#[allow(clippy::type_complexity)]
pub fn new_partial(
    config: &Configuration,
) -> Result<
    sc_service::PartialComponents<
        FullClient,
        FullBackend,
        FullSelectChain,
        sp_consensus::DefaultImportQueue<Block, FullClient>,
        sc_transaction_pool::FullPool<Block, FullClient>,
        sc_consensus_aura::AuraBlockImport<
            Block,
            FullClient,
            AlephBlockImport<Block, FullBackend, FullClient>,
            AuraPair,
        >,
    >,
    ServiceError,
> {
    let inherent_data_providers = InherentDataProviders::new();

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

    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts::<Block, RuntimeApi, Executor>(
            &config,
            telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
        )?;
    let client: Arc<TFullClient<_, _, _>> = Arc::new(client);

    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        config.role.is_authority().into(),
        config.prometheus_registry(),
        task_manager.spawn_handle(),
        client.clone(),
    );

    let (_, authorities) = get_authorities(client.clone(), keystore_container.sync_keystore());
    let aleph_block_import = AlephBlockImport::new(client.clone() as Arc<_>, authorities);

    let aura_block_import = sc_consensus_aura::AuraBlockImport::<_, _, _, AuraPair>::new(
        aleph_block_import.clone(),
        client.clone(),
    );

    let import_queue =
        sc_consensus_aura::import_queue::<AuraPair, _, _, _, _, _>(ImportQueueParams {
            block_import: aura_block_import.clone(),
            justification_import: Some(Box::new(aleph_block_import)),
            client: client.clone(),
            inherent_data_providers: inherent_data_providers.clone(),
            spawner: &task_manager.spawn_essential_handle(),
            registry: config.prometheus_registry(),
            can_author_with: sp_consensus::CanAuthorWithNativeVersion::new(
                client.executor().clone(),
            ),
            check_for_equivocation: Default::default(),
            slot_duration: sc_consensus_aura::slot_duration(&*client)?,
            telemetry: telemetry.as_ref().map(|(_, x)| x.handle()),
        })?;

    Ok(sc_service::PartialComponents {
        client,
        backend,
        task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool,
        inherent_data_providers,
        other: aura_block_import,
    })
}

fn get_authorities(
    client: Arc<FullClient>,
    keystore: SyncCryptoStorePtr,
) -> (AuthorityId, Vec<AuthorityId>) {
    let auth = SyncCryptoStore::sr25519_public_keys(&*keystore, finality_aleph::KEY_TYPE)[0];
    let authorities = client
        .executor()
        .call(
            &BlockId::Number(Zero::zero()),
            "AuraApi_authorities",
            &[],
            ExecutionStrategy::NativeElseWasm,
            None,
        )
        .ok()
        .map(|call_result| Vec::<AuthorityId>::decode(&mut &call_result[..]).unwrap())
        .unwrap();

    (auth.into(), authorities)
}

fn consensus_config(auth: AuthorityId, authorities: &[AuthorityId]) -> ConsensusConfig {
    let node_id = authorities.iter().position(|a| a == &auth).unwrap().into();
    let n_members = authorities.len().into();

    ConsensusConfig {
        node_id,
        session_id: 0,
        n_members,
        create_lag: std::time::Duration::from_millis(500),
    }
}

/// Builds a new service for a full client.
pub fn new_full(mut config: Configuration) -> Result<TaskManager, ServiceError> {
    let sc_service::PartialComponents {
        client,
        backend,
        mut task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool,
        inherent_data_providers,
        other: block_import,
        ..
    } = new_partial(&config)?;

    config
        .network
        .extra_sets
        .push(finality_aleph::peers_set_config());

    let (network, network_status_sinks, system_rpc_tx, network_starter) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            on_demand: None,
            block_announce_validator_builder: None,
        })?;

    let role = config.role.clone();
    let force_authoring = config.force_authoring;
    let backoff_authoring_blocks: Option<()> = None;
    let prometheus_registry = config.prometheus_registry().cloned();
    let (authority_id, authorities) =
        get_authorities(client.clone(), keystore_container.sync_keystore());

    let rpc_extensions_builder = {
        let client = client.clone();
        let pool = transaction_pool.clone();

        Box::new(move |deny_unsafe, _| {
            let deps = crate::rpc::FullDeps {
                client: client.clone(),
                pool: pool.clone(),
                deny_unsafe,
            };

            crate::rpc::create_full(deps)
        })
    };

    let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        config,
        client: client.clone(),
        backend,
        task_manager: &mut task_manager,
        keystore: keystore_container.sync_keystore(),
        on_demand: None,
        transaction_pool: transaction_pool.clone(),
        rpc_extensions_builder,
        remote_blockchain: None,
        network: network.clone(),
        network_status_sinks,
        system_rpc_tx,
        telemetry: None,
    })?;

    if role.is_authority() {
        let proposer_factory = sc_basic_authorship::ProposerFactory::new(
            task_manager.spawn_handle(),
            client.clone(),
            transaction_pool,
            prometheus_registry.as_ref(),
            None,
        );

        let can_author_with =
            sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

        let aura = sc_consensus_aura::start_aura::<AuraPair, _, _, _, _, _, _, _, _, _>(
            StartAuraParams {
                slot_duration: sc_consensus_aura::slot_duration(&*client)?,
                client: client.clone(),
                select_chain: select_chain.clone(),
                block_import,
                proposer_factory,
                sync_oracle: network.clone(),
                inherent_data_providers,
                force_authoring,
                backoff_authoring_blocks,
                keystore: keystore_container.sync_keystore(),
                can_author_with,
                block_proposal_slot_portion: SlotProportion::new(2f32 / 3f32),
                telemetry: None,
            },
        )?;

        task_manager
            .spawn_essential_handle()
            .spawn_blocking("aura", aura);

        let aleph_config = AlephConfig {
            network,
            consensus_config: consensus_config(authority_id.clone(), &authorities),
            client,
            select_chain,
            spawn_handle: task_manager.spawn_handle(),
            auth_keystore: AuthorityKeystore::new(authority_id, keystore_container.sync_keystore()),
            authorities,
        };
        task_manager
            .spawn_essential_handle()
            .spawn_blocking("aleph", run_aleph_consensus(aleph_config));
    }

    network_starter.start_network();
    Ok(task_manager)
}
