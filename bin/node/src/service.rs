//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use aleph_runtime::{self, opaque::Block, RuntimeApi};
use finality_aleph::{
    run_aleph_consensus, AlephConfig, AuthorityId, AuthorityKeystore, ConsensusConfig, EpochId,
    NodeId,
};
use sc_client_api::ExecutorProvider;
use sc_executor::native_executor_instance;
pub use sc_executor::NativeExecutor;
use sc_service::{error::Error as ServiceError, Configuration, TaskManager};
use sp_consensus_aura::sr25519::AuthorityPair as AuraPair;
use sp_core::{Pair, Public};
use sp_inherents::InherentDataProviders;
use sp_keystore::{SyncCryptoStore, SyncCryptoStorePtr};
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

pub fn new_partial(
    config: &Configuration,
) -> Result<
    sc_service::PartialComponents<
        FullClient,
        FullBackend,
        FullSelectChain,
        sp_consensus::DefaultImportQueue<Block, FullClient>,
        sc_transaction_pool::FullPool<Block, FullClient>,
        sc_consensus_aura::AuraBlockImport<Block, FullClient, Arc<FullClient>, AuraPair>,
    >,
    ServiceError,
> {
    let inherent_data_providers = InherentDataProviders::new();

    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts::<Block, RuntimeApi, Executor>(&config)?;
    let client = Arc::new(client);

    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        config.role.is_authority().into(),
        config.prometheus_registry(),
        task_manager.spawn_handle(),
        client.clone(),
    );

    let aura_block_import = sc_consensus_aura::AuraBlockImport::<_, _, _, AuraPair>::new(
        client.clone(),
        client.clone(),
    );

    let import_queue = sc_consensus_aura::import_queue::<_, _, _, AuraPair, _, _>(
        sc_consensus_aura::slot_duration(&*client)?,
        aura_block_import.clone(),
        None,
        client.clone(),
        inherent_data_providers.clone(),
        &task_manager.spawn_handle(),
        config.prometheus_registry(),
        sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone()),
    )?;

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

pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

fn get_authorities(
    config: &Configuration,
    keystore: SyncCryptoStorePtr,
) -> (AuthorityId, Vec<AuthorityId>) {
    let key_type_id = finality_aleph::KEY_TYPE;
    let name = config.network.node_name.clone();
    let seed = format!("//{}", name);
    let keys = SyncCryptoStore::sr25519_public_keys(&*keystore, key_type_id);

    let our_key = if keys.is_empty() {
        SyncCryptoStore::sr25519_generate_new(&*keystore, key_type_id, Some(&seed))
            .unwrap()
            .into()
    } else {
        panic!(
            "For some reason the key is already in the keystore. Make sure you clear the keystore."
        )
    };
    (
        our_key,
        vec![
            get_from_seed::<AuthorityId>("Alice"),
            get_from_seed::<AuthorityId>("Bob"),
        ],
    )
}

fn consensus_config(config: &Configuration, auth: AuthorityId) -> ConsensusConfig<NodeId> {
    let name = config.network.node_name.clone();
    let node_id = NodeId {
        auth,
        // TODO add index calculation based on order on keys
        index: match name.as_str() {
            "Alice" => 0.into(),
            "Bob" => 1.into(),
            _ => panic!("unknown identity"),
        },
    };
    let n_members = 2.into();

    ConsensusConfig::new(
        node_id,
        n_members,
        EpochId(0),
        std::time::Duration::from_millis(500),
    )
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
    let (authority_id, authorities) = get_authorities(&config, keystore_container.sync_keystore());
    let consensus_config = consensus_config(&config, authority_id.clone());

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

    let (_rpc_handlers, _maybe_telemetry) =
        sc_service::spawn_tasks(sc_service::SpawnTasksParams {
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
        })?;

    if role.is_authority() {
        let proposer_factory = sc_basic_authorship::ProposerFactory::new(
            task_manager.spawn_handle(),
            client.clone(),
            transaction_pool,
            prometheus_registry.as_ref(),
        );

        let can_author_with =
            sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

        let aura = sc_consensus_aura::start_aura::<_, _, _, _, _, AuraPair, _, _, _, _>(
            sc_consensus_aura::slot_duration(&*client)?,
            client.clone(),
            select_chain.clone(),
            block_import,
            proposer_factory,
            network.clone(),
            inherent_data_providers,
            force_authoring,
            backoff_authoring_blocks,
            keystore_container.sync_keystore(),
            can_author_with,
        )?;

        task_manager
            .spawn_essential_handle()
            .spawn_blocking("aura", aura);

        let aleph_config = AlephConfig {
            network,
            consensus_config,
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
