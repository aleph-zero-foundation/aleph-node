use std::{collections::HashSet, marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use futures::channel::oneshot;
use log::{debug, info, trace, warn};
use network_clique::SpawnHandleExt;
use pallet_aleph_runtime_api::AlephSessionApi;
use sc_keystore::{Keystore, LocalKeystore};
use sp_application_crypto::RuntimeAppPublic;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};

use crate::{
    abft::{
        current_create_aleph_config, legacy_create_aleph_config, run_current_member,
        run_legacy_member, SpawnHandle,
    },
    aleph_primitives::{BlockHash, BlockNumber, KEY_TYPE},
    block::{
        substrate::{Justification, JustificationTranslator},
        BestBlockSelector, Block, Header, HeaderVerifier, UnverifiedHeader,
    },
    crypto::{AuthorityPen, AuthorityVerifier},
    data_io::{ChainTracker, DataStore, OrderedDataInterpreter, SubstrateChainInfoProvider},
    metrics::TimingBlockMetrics,
    mpsc,
    network::{
        data::{
            component::{Network, NetworkMap, SimpleNetwork},
            split::split,
        },
        session::{SessionManager, SessionSender},
    },
    party::{
        backup::ABFTBackup, manager::aggregator::AggregatorVersion, traits::NodeSessionManager,
    },
    sync::JustificationSubmissions,
    AuthorityId, BlockId, CurrentRmcNetworkData, Keychain, LegacyRmcNetworkData, NodeIndex,
    ProvideRuntimeApi, SessionBoundaries, SessionBoundaryInfo, SessionId, SessionPeriod,
    UnitCreationDelay, VersionedNetworkData,
};

mod aggregator;
mod authority;
mod task;

pub use authority::{Subtasks, Task as AuthorityTask};
pub use task::{Handle, Runnable, Task, TaskCommon};

use crate::{
    abft::{CURRENT_VERSION, LEGACY_VERSION},
    block::{BlockchainEvents, HeaderBackend},
    sync::RequestBlocks,
};

#[cfg(feature = "only_legacy")]
const ONLY_LEGACY_ENV: &str = "ONLY_LEGACY_PROTOCOL";

type LegacyNetworkType = SimpleNetwork<
    LegacyRmcNetworkData,
    mpsc::UnboundedReceiver<LegacyRmcNetworkData>,
    SessionSender<LegacyRmcNetworkData>,
>;
type CurrentNetworkType = SimpleNetwork<
    CurrentRmcNetworkData,
    mpsc::UnboundedReceiver<CurrentRmcNetworkData>,
    SessionSender<CurrentRmcNetworkData>,
>;

struct SubtasksParams<H, HB, N, JS>
where
    H: Header,
    HB: HeaderBackend<H> + Send + Sync + 'static,
    N: Network<VersionedNetworkData<H::Unverified>> + 'static,
    JS: JustificationSubmissions<Justification> + Send + Sync + Clone,
{
    n_members: usize,
    node_id: NodeIndex,
    session_id: SessionId,
    data_network: N,
    session_boundaries: SessionBoundaries,
    subtask_common: TaskCommon,
    blocks_for_aggregator: mpsc::UnboundedSender<BlockId>,
    chain_info: SubstrateChainInfoProvider<H, HB>,
    aggregator_io: aggregator::IO<JS>,
    multikeychain: Keychain,
    exit_rx: oneshot::Receiver<()>,
    backup: ABFTBackup,
}

pub struct NodeSessionManagerImpl<H, C, HB, BBS, B, RB, SM, JS, V>
where
    H: Header,
    B: Block<UnverifiedHeader = H::Unverified> + BlockT<Hash = BlockHash>,
    B::Header: HeaderT<Number = BlockNumber> + UnverifiedHeader,
    C: ProvideRuntimeApi<B> + BlockchainEvents<H> + Send + Sync + 'static,
    C::Api: AlephSessionApi<B>,
    HB: HeaderBackend<H> + Send + Sync + 'static,
    BBS: BestBlockSelector<H> + 'static,
    RB: RequestBlocks<B::UnverifiedHeader>,
    SM: SessionManager<VersionedNetworkData<B::UnverifiedHeader>> + 'static,
    JS: JustificationSubmissions<Justification> + Send + Sync + Clone,
    V: HeaderVerifier<H>,
{
    client: Arc<C>,
    header_backend: HB,
    best_block_selection_strategy: BBS,
    verifier: V,
    session_info: SessionBoundaryInfo,
    unit_creation_delay: UnitCreationDelay,
    justifications_for_sync: JS,
    justification_translator: JustificationTranslator,
    block_requester: RB,
    metrics: TimingBlockMetrics,
    spawn_handle: SpawnHandle,
    session_manager: SM,
    keystore: Arc<LocalKeystore>,
    _phantom: PhantomData<(B, H)>,
}

impl<H, C, HB, BBS, B, RB, SM, JS, V> NodeSessionManagerImpl<H, C, HB, BBS, B, RB, SM, JS, V>
where
    H: Header,
    B: Block<UnverifiedHeader = H::Unverified> + BlockT<Hash = BlockHash>,
    B::Header: HeaderT<Number = BlockNumber> + UnverifiedHeader,
    C: ProvideRuntimeApi<B> + BlockchainEvents<H> + Send + Sync + 'static,
    C::Api: AlephSessionApi<B>,
    HB: HeaderBackend<H> + Send + Sync + Clone + 'static,
    BBS: BestBlockSelector<H> + 'static,
    RB: RequestBlocks<B::UnverifiedHeader>,
    SM: SessionManager<VersionedNetworkData<B::UnverifiedHeader>> + 'static,
    JS: JustificationSubmissions<Justification> + Send + Sync + Clone,
    V: HeaderVerifier<H>,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        client: Arc<C>,
        header_backend: HB,
        best_block_selection_strategy: BBS,
        verifier: V,
        session_period: SessionPeriod,
        unit_creation_delay: UnitCreationDelay,
        justifications_for_sync: JS,
        justification_translator: JustificationTranslator,
        block_requester: RB,
        metrics: TimingBlockMetrics,
        spawn_handle: SpawnHandle,
        session_manager: SM,
        keystore: Arc<LocalKeystore>,
    ) -> Self {
        Self {
            client,
            header_backend,
            best_block_selection_strategy,
            verifier,
            session_info: SessionBoundaryInfo::new(session_period),
            unit_creation_delay,
            justifications_for_sync,
            justification_translator,
            block_requester,
            metrics,
            spawn_handle,
            session_manager,
            keystore,
            _phantom: PhantomData,
        }
    }

    fn legacy_subtasks<N: Network<VersionedNetworkData<B::UnverifiedHeader>> + 'static>(
        &self,
        params: SubtasksParams<H, HB, N, JS>,
    ) -> Subtasks {
        let SubtasksParams {
            n_members,
            node_id,
            session_id,
            data_network,
            session_boundaries,
            subtask_common,
            blocks_for_aggregator,
            chain_info,
            aggregator_io,
            multikeychain,
            exit_rx,
            backup,
            ..
        } = params;
        let (chain_tracker, data_provider) = ChainTracker::new(
            self.best_block_selection_strategy.clone(),
            self.header_backend.clone(),
            session_boundaries.clone(),
            Default::default(),
            self.metrics.clone(),
        );
        let ordered_data_interpreter = OrderedDataInterpreter::new(
            blocks_for_aggregator,
            chain_info,
            self.verifier.clone(),
            session_boundaries.clone(),
        );
        let consensus_config =
            legacy_create_aleph_config(n_members, node_id, session_id, self.unit_creation_delay);
        let data_network = data_network.map();

        let (unfiltered_aleph_network, rmc_network) =
            split(data_network, "aleph_network", "rmc_network");
        let (data_store, aleph_network) = DataStore::new(
            session_boundaries.clone(),
            self.header_backend.clone(),
            self.client.clone(),
            self.verifier.clone(),
            self.block_requester.clone(),
            Default::default(),
            unfiltered_aleph_network,
        );
        Subtasks::new(
            exit_rx,
            run_legacy_member(
                subtask_common.clone(),
                multikeychain.clone(),
                consensus_config,
                aleph_network.into(),
                data_provider,
                ordered_data_interpreter,
                backup,
            ),
            aggregator::task(
                subtask_common.clone(),
                self.header_backend.clone(),
                aggregator_io,
                session_boundaries,
                self.metrics.clone(),
                multikeychain,
                AggregatorVersion::<CurrentNetworkType, _>::Legacy(rmc_network),
            ),
            task::task(subtask_common.clone(), chain_tracker, "chain tracker"),
            task::task(subtask_common, data_store, "data store"),
        )
    }

    fn current_subtasks<N: Network<VersionedNetworkData<B::UnverifiedHeader>> + 'static>(
        &self,
        params: SubtasksParams<H, HB, N, JS>,
    ) -> Subtasks {
        let SubtasksParams {
            n_members,
            node_id,
            session_id,
            data_network,
            session_boundaries,
            subtask_common,
            blocks_for_aggregator,
            chain_info,
            aggregator_io,
            multikeychain,
            exit_rx,
            backup,
            ..
        } = params;
        let (chain_tracker, data_provider) = ChainTracker::new(
            self.best_block_selection_strategy.clone(),
            self.header_backend.clone(),
            session_boundaries.clone(),
            Default::default(),
            self.metrics.clone(),
        );
        let ordered_data_interpreter = OrderedDataInterpreter::new(
            blocks_for_aggregator,
            chain_info,
            self.verifier.clone(),
            session_boundaries.clone(),
        );
        let consensus_config =
            current_create_aleph_config(n_members, node_id, session_id, self.unit_creation_delay);
        let data_network = data_network.map();

        let (unfiltered_aleph_network, rmc_network) =
            split(data_network, "aleph_network", "rmc_network");
        let (data_store, aleph_network) = DataStore::new(
            session_boundaries.clone(),
            self.header_backend.clone(),
            self.client.clone(),
            self.verifier.clone(),
            self.block_requester.clone(),
            Default::default(),
            unfiltered_aleph_network,
        );
        Subtasks::new(
            exit_rx,
            run_current_member(
                subtask_common.clone(),
                multikeychain.clone(),
                consensus_config,
                aleph_network.into(),
                data_provider,
                ordered_data_interpreter,
                backup,
            ),
            aggregator::task(
                subtask_common.clone(),
                self.header_backend.clone(),
                aggregator_io,
                session_boundaries,
                self.metrics.clone(),
                multikeychain,
                AggregatorVersion::<_, LegacyNetworkType>::Current(rmc_network),
            ),
            task::task(subtask_common.clone(), chain_tracker, "chain tracker"),
            task::task(subtask_common, data_store, "data store"),
        )
    }

    async fn spawn_subtasks(
        &self,
        session_id: SessionId,
        authorities: &[AuthorityId],
        node_id: NodeIndex,
        exit_rx: oneshot::Receiver<()>,
        backup: ABFTBackup,
    ) -> Subtasks {
        debug!(target: "afa", "Authority task {:?}", session_id);

        let authority_verifier = AuthorityVerifier::new(authorities.to_vec());
        let authority_pen =
            AuthorityPen::new(authorities[node_id.0].clone(), self.keystore.clone())
                .expect("The keys should sign successfully");
        let multikeychain =
            Keychain::new(node_id, authority_verifier.clone(), authority_pen.clone());

        let session_boundaries = self.session_info.boundaries_for_session(session_id);
        let (blocks_for_aggregator, blocks_from_interpreter) = mpsc::unbounded();

        let chain_info = SubstrateChainInfoProvider::new(self.header_backend.clone());

        let subtask_common = TaskCommon {
            spawn_handle: self.spawn_handle.clone(),
            session_id: session_id.0,
        };
        let aggregator_io = aggregator::IO {
            blocks_from_interpreter,
            justifications_for_chain: self.justifications_for_sync.clone(),
            justification_translator: self.justification_translator.clone(),
        };

        let data_network = match self
            .session_manager
            .start_validator_session(session_id, authority_verifier, node_id, authority_pen)
            .await
        {
            Ok(data_network) => data_network,
            Err(e) => panic!("Failed to start validator session: {e}"),
        };

        let last_block_of_previous_session = session_boundaries.first_block().saturating_sub(1);
        let last_block_of_previous_session_hash = self
            .header_backend
            .header_of_finalized_at(last_block_of_previous_session)
            .expect("Previous session ended, the block should be present.")
            .expect("Previous session ended, the block should be finalized.")
            .id()
            .hash();

        let params = SubtasksParams {
            n_members: authorities.len(),
            node_id,
            session_id,
            data_network,
            session_boundaries,
            subtask_common,
            blocks_for_aggregator,
            chain_info,
            aggregator_io,
            multikeychain,
            exit_rx,
            backup,
        };

        match self
            .client
            .runtime_api()
            .next_session_finality_version(last_block_of_previous_session_hash)
        {
            #[cfg(feature = "only_legacy")]
            _ if self.only_legacy() => {
                info!(target: "aleph-party", "Running session with legacy-only AlephBFT version.");
                self.legacy_subtasks(params)
            }
            // The `as`es here should be removed, but this would require a pallet migration and I
            // am lazy.
            Ok(version) if version == CURRENT_VERSION as u32 => {
                info!(target: "aleph-party", "Running session with AlephBFT version {}, which is current.", version);
                self.current_subtasks(params)
            }
            Ok(version) if version == LEGACY_VERSION as u32 => {
                info!(target: "aleph-party", "Running session with AlephBFT version {}, which is legacy.", version);
                self.legacy_subtasks(params)
            }
            Ok(version) if version > CURRENT_VERSION as u32 => {
                panic!(
                    "Too new version {version}. Supported versions: {LEGACY_VERSION} or {CURRENT_VERSION}. Probably outdated node."
                )
            }
            Ok(version) => {
                info!(target: "aleph-party", "Attempting to run session with too old version {}, likely because we are synchronizing old sessions for which we have keys. This will not work, but it doesn't matter.", version);
                info!(target: "aleph-party", "Running session with AlephBFT version {}, which is legacy.", LEGACY_VERSION);
                self.legacy_subtasks(params)
            }
            _ => {
                // this might happen when there was no runtime upgrade yet. Fallback to legacy version
                self.legacy_subtasks(params)
            }
        }
    }

    #[cfg(feature = "only_legacy")]
    fn only_legacy(&self) -> bool {
        std::env::var(ONLY_LEGACY_ENV)
            .map(|legacy| !legacy.is_empty())
            .unwrap_or(false)
    }
}

#[async_trait]
impl<H, C, HB, BBS, B, RB, SM, JS, V> NodeSessionManager
    for NodeSessionManagerImpl<H, C, HB, BBS, B, RB, SM, JS, V>
where
    H: Header,
    B: Block<UnverifiedHeader = H::Unverified> + BlockT<Hash = BlockHash>,
    B::Header: HeaderT<Number = BlockNumber> + UnverifiedHeader,
    C: ProvideRuntimeApi<B> + BlockchainEvents<H> + Send + Sync + 'static,
    C::Api: AlephSessionApi<B>,
    HB: HeaderBackend<H> + Send + Sync + Clone + 'static,
    BBS: BestBlockSelector<H> + 'static,
    RB: RequestBlocks<B::UnverifiedHeader>,
    SM: SessionManager<VersionedNetworkData<B::UnverifiedHeader>> + 'static,
    JS: JustificationSubmissions<Justification> + Send + Sync + Clone,
    V: HeaderVerifier<H>,
{
    type Error = SM::Error;

    async fn spawn_authority_task_for_session(
        &self,
        session: SessionId,
        node_id: NodeIndex,
        backup: ABFTBackup,
        authorities: &[AuthorityId],
    ) -> AuthorityTask {
        let (exit, exit_rx) = futures::channel::oneshot::channel();
        let subtasks = self
            .spawn_subtasks(session, authorities, node_id, exit_rx, backup)
            .await;

        AuthorityTask::new(
            self.spawn_handle
                .spawn_essential("aleph/session_authority", async move {
                    if subtasks.wait_completion().await.is_err() {
                        warn!(target: "aleph-party", "Authority subtasks failed.");
                    }
                }),
            node_id,
            exit,
        )
    }

    fn early_start_validator_session(
        &self,
        session: SessionId,
        node_id: NodeIndex,
        authorities: &[AuthorityId],
    ) -> Result<(), Self::Error> {
        let authority_verifier = AuthorityVerifier::new(authorities.to_vec());
        let authority_pen =
            AuthorityPen::new(authorities[node_id.0].clone(), self.keystore.clone())
                .expect("The keys should sign successfully");
        self.session_manager.early_start_validator_session(
            session,
            authority_verifier,
            node_id,
            authority_pen,
        )
    }

    fn start_nonvalidator_session(
        &self,
        session: SessionId,
        authorities: &[AuthorityId],
    ) -> Result<(), Self::Error> {
        let authority_verifier = AuthorityVerifier::new(authorities.to_vec());

        self.session_manager
            .start_nonvalidator_session(session, authority_verifier)
    }

    fn stop_session(&self, session: SessionId) -> Result<(), Self::Error> {
        self.session_manager.stop_session(session)
    }

    fn node_idx(&self, authorities: &[AuthorityId]) -> Option<NodeIndex> {
        let our_consensus_keys: HashSet<_> = match self.keystore.keys(KEY_TYPE) {
            Ok(keys) => keys.into_iter().collect(),
            Err(e) => {
                warn!(target: "aleph-data-store", "Error accessing keystore: {}", e);
                return None;
            }
        };
        trace!(target: "aleph-data-store", "Found {:?} consensus keys in our local keystore {:?}", our_consensus_keys.len(), our_consensus_keys);
        authorities
            .iter()
            .position(|pkey| our_consensus_keys.contains(&pkey.to_raw_vec()))
            .map(|id| id.into())
    }
}
