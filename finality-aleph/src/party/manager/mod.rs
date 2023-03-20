use std::{collections::HashSet, marker::PhantomData, sync::Arc};

use aleph_primitives::{AlephSessionApi, BlockNumber, KEY_TYPE};
use async_trait::async_trait;
use futures::channel::oneshot;
use log::{debug, info, trace, warn};
use sc_client_api::Backend;
use sp_consensus::SelectChain;
use sp_keystore::CryptoStore;
use sp_runtime::{
    generic::BlockId,
    traits::{Block as BlockT, Header as HeaderT},
};

use crate::{
    abft::{
        current_create_aleph_config, legacy_create_aleph_config, run_current_member,
        run_legacy_member, SpawnHandle, SpawnHandleT,
    },
    crypto::{AuthorityPen, AuthorityVerifier},
    data_io::{ChainTracker, DataStore, OrderedDataInterpreter},
    mpsc,
    network::{
        data::{
            component::{Network, NetworkMap, SimpleNetwork},
            split::split,
        },
        session::{SessionManager, SessionSender},
        RequestBlocks,
    },
    party::{
        backup::ABFTBackup, manager::aggregator::AggregatorVersion, traits::NodeSessionManager,
    },
    AuthorityId, CurrentRmcNetworkData, JustificationNotification, Keychain, LegacyRmcNetworkData,
    Metrics, NodeIndex, SessionBoundaries, SessionBoundaryInfo, SessionId, SessionPeriod,
    UnitCreationDelay, VersionedNetworkData,
};

mod aggregator;
mod authority;
mod chain_tracker;
mod data_store;
mod task;

pub use authority::{SubtaskCommon, Subtasks, Task as AuthorityTask};
pub use task::{Handle, Task};

use crate::{
    abft::{CURRENT_VERSION, LEGACY_VERSION},
    data_io::DataProvider,
};

#[cfg(feature = "only_legacy")]
const ONLY_LEGACY_ENV: &str = "ONLY_LEGACY_PROTOCOL";

type LegacyNetworkType<B> = SimpleNetwork<
    LegacyRmcNetworkData<B>,
    mpsc::UnboundedReceiver<LegacyRmcNetworkData<B>>,
    SessionSender<LegacyRmcNetworkData<B>>,
>;
type CurrentNetworkType<B> = SimpleNetwork<
    CurrentRmcNetworkData<B>,
    mpsc::UnboundedReceiver<CurrentRmcNetworkData<B>>,
    SessionSender<CurrentRmcNetworkData<B>>,
>;

struct SubtasksParams<C, SC, B, N, BE>
where
    B: BlockT,
    B::Header: HeaderT<Number = BlockNumber>,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
    N: Network<VersionedNetworkData<B>> + 'static,
{
    n_members: usize,
    node_id: NodeIndex,
    session_id: SessionId,
    data_network: N,
    session_boundaries: SessionBoundaries,
    subtask_common: SubtaskCommon,
    data_provider: DataProvider<B>,
    ordered_data_interpreter: OrderedDataInterpreter<B, C>,
    aggregator_io: aggregator::IO<B>,
    multikeychain: Keychain,
    exit_rx: oneshot::Receiver<()>,
    backup: ABFTBackup,
    chain_tracker: ChainTracker<B, SC, C>,
    phantom: PhantomData<BE>,
}

pub struct NodeSessionManagerImpl<C, SC, B, RB, BE, SM>
where
    B: BlockT,
    B::Header: HeaderT<Number = BlockNumber>,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
    RB: RequestBlocks<B>,
    SM: SessionManager<VersionedNetworkData<B>> + 'static,
{
    client: Arc<C>,
    select_chain: SC,
    session_info: SessionBoundaryInfo,
    unit_creation_delay: UnitCreationDelay,
    authority_justification_tx: mpsc::UnboundedSender<JustificationNotification<B>>,
    block_requester: RB,
    metrics: Option<Metrics<<B::Header as HeaderT>::Hash>>,
    spawn_handle: SpawnHandle,
    session_manager: SM,
    keystore: Arc<dyn CryptoStore>,
    _phantom: PhantomData<BE>,
}

impl<C, SC, B, RB, BE, SM> NodeSessionManagerImpl<C, SC, B, RB, BE, SM>
where
    B: BlockT,
    B::Header: HeaderT<Number = BlockNumber>,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B>,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
    RB: RequestBlocks<B>,
    SM: SessionManager<VersionedNetworkData<B>>,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        client: Arc<C>,
        select_chain: SC,
        session_period: SessionPeriod,
        unit_creation_delay: UnitCreationDelay,
        authority_justification_tx: mpsc::UnboundedSender<JustificationNotification<B>>,
        block_requester: RB,
        metrics: Option<Metrics<<B::Header as HeaderT>::Hash>>,
        spawn_handle: SpawnHandle,
        session_manager: SM,
        keystore: Arc<dyn CryptoStore>,
    ) -> Self {
        Self {
            client,
            select_chain,
            session_info: SessionBoundaryInfo::new(session_period),
            unit_creation_delay,
            authority_justification_tx,
            block_requester,
            metrics,
            spawn_handle,
            session_manager,
            keystore,
            _phantom: PhantomData,
        }
    }

    fn legacy_subtasks<N: Network<VersionedNetworkData<B>> + 'static>(
        &self,
        params: SubtasksParams<C, SC, B, N, BE>,
    ) -> Subtasks {
        let SubtasksParams {
            n_members,
            node_id,
            session_id,
            data_network,
            session_boundaries,
            subtask_common,
            data_provider,
            ordered_data_interpreter,
            aggregator_io,
            multikeychain,
            exit_rx,
            backup,
            chain_tracker,
            ..
        } = params;
        let consensus_config =
            legacy_create_aleph_config(n_members, node_id, session_id, self.unit_creation_delay);
        let data_network = data_network.map();

        let (unfiltered_aleph_network, rmc_network) =
            split(data_network, "aleph_network", "rmc_network");
        let (data_store, aleph_network) = DataStore::new(
            session_boundaries.clone(),
            self.client.clone(),
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
                self.client.clone(),
                aggregator_io,
                session_boundaries,
                self.metrics.clone(),
                multikeychain,
                AggregatorVersion::<CurrentNetworkType<B>, _>::Legacy(rmc_network),
            ),
            chain_tracker::task(subtask_common.clone(), chain_tracker),
            data_store::task(subtask_common, data_store),
        )
    }

    fn current_subtasks<N: Network<VersionedNetworkData<B>> + 'static>(
        &self,
        params: SubtasksParams<C, SC, B, N, BE>,
    ) -> Subtasks {
        let SubtasksParams {
            n_members,
            node_id,
            session_id,
            data_network,
            session_boundaries,
            subtask_common,
            data_provider,
            ordered_data_interpreter,
            aggregator_io,
            multikeychain,
            exit_rx,
            backup,
            chain_tracker,
            ..
        } = params;
        let consensus_config =
            current_create_aleph_config(n_members, node_id, session_id, self.unit_creation_delay);
        let data_network = data_network.map();

        let (unfiltered_aleph_network, rmc_network) =
            split(data_network, "aleph_network", "rmc_network");
        let (data_store, aleph_network) = DataStore::new(
            session_boundaries.clone(),
            self.client.clone(),
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
                self.client.clone(),
                aggregator_io,
                session_boundaries,
                self.metrics.clone(),
                multikeychain,
                AggregatorVersion::<_, LegacyNetworkType<B>>::Current(rmc_network),
            ),
            chain_tracker::task(subtask_common.clone(), chain_tracker),
            data_store::task(subtask_common, data_store),
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
                .await
                .expect("The keys should sign successfully");
        let multikeychain =
            Keychain::new(node_id, authority_verifier.clone(), authority_pen.clone());

        let session_boundaries = self.session_info.boundaries_for_session(session_id);
        let (blocks_for_aggregator, blocks_from_interpreter) = mpsc::unbounded();

        let (chain_tracker, data_provider) = ChainTracker::new(
            self.select_chain.clone(),
            self.client.clone(),
            session_boundaries.clone(),
            Default::default(),
            self.metrics.clone(),
        );

        let ordered_data_interpreter = OrderedDataInterpreter::<B, C>::new(
            blocks_for_aggregator,
            self.client.clone(),
            session_boundaries.clone(),
        );

        let subtask_common = SubtaskCommon {
            spawn_handle: self.spawn_handle.clone(),
            session_id: session_id.0,
        };
        let aggregator_io = aggregator::IO {
            blocks_from_interpreter,
            justifications_for_chain: self.authority_justification_tx.clone(),
        };

        let data_network = match self
            .session_manager
            .start_validator_session(session_id, authority_verifier, node_id, authority_pen)
            .await
        {
            Ok(data_network) => data_network,
            Err(e) => panic!("Failed to start validator session: {}", e),
        };

        let last_block_of_previous_session = session_boundaries.first_block().saturating_sub(1);

        let params = SubtasksParams {
            n_members: authorities.len(),
            node_id,
            session_id,
            data_network,
            session_boundaries,
            subtask_common,
            data_provider,
            ordered_data_interpreter,
            aggregator_io,
            multikeychain,
            exit_rx,
            backup,
            chain_tracker,
            phantom: PhantomData,
        };

        match self
            .client
            .runtime_api()
            .next_session_finality_version(&BlockId::Number(last_block_of_previous_session))
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
            Ok(version) => {
                panic!("Unsupported version {}. Supported versions: {} or {}. Potentially outdated node.", version, LEGACY_VERSION, CURRENT_VERSION)
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
impl<C, SC, B, RB, BE, SM> NodeSessionManager for NodeSessionManagerImpl<C, SC, B, RB, BE, SM>
where
    B: BlockT,
    B::Header: HeaderT<Number = BlockNumber>,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B>,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
    RB: RequestBlocks<B>,
    SM: SessionManager<VersionedNetworkData<B>>,
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

    async fn early_start_validator_session(
        &self,
        session: SessionId,
        node_id: NodeIndex,
        authorities: &[AuthorityId],
    ) -> Result<(), Self::Error> {
        let authority_verifier = AuthorityVerifier::new(authorities.to_vec());
        let authority_pen =
            AuthorityPen::new(authorities[node_id.0].clone(), self.keystore.clone())
                .await
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

    async fn node_idx(&self, authorities: &[AuthorityId]) -> Option<NodeIndex> {
        let our_consensus_keys: HashSet<_> = self
            .keystore
            .keys(KEY_TYPE)
            .await
            .unwrap()
            .into_iter()
            .collect();
        trace!(target: "aleph-data-store", "Found {:?} consensus keys in our local keystore {:?}", our_consensus_keys.len(), our_consensus_keys);
        authorities
            .iter()
            .position(|pkey| our_consensus_keys.contains(&pkey.into()))
            .map(|id| id.into())
    }
}
