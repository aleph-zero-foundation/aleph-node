use crate::{
    crypto::{AuthorityPen, AuthorityVerifier, KeyBox},
    data_io::{ChainTracker, DataStore, OrderedDataInterpreter},
    default_aleph_config,
    finalization::{AlephFinalizer, BlockFinalizer},
    first_block_of_session,
    justification::{
        AlephJustification, JustificationHandler, JustificationNotification,
        JustificationRequestScheduler, JustificationRequestSchedulerImpl, SessionInfo,
        SessionInfoProvider, Verifier,
    },
    last_block_of_session,
    network::{
        split, AlephNetworkData, ConnectionIO, ConnectionManager, ConnectionManagerConfig,
        RequestBlocks, RmcNetworkData, Service as NetworkService, SessionManager, SessionNetwork,
        Split, IO as NetworkIO,
    },
    session_id_from_block_num, AuthorityId, Metrics, NodeIndex, SessionBoundaries, SessionId,
    SessionMap, SessionPeriod, UnitCreationDelay,
};
use sp_keystore::CryptoStore;

use aleph_bft::{DelayConfig, SpawnHandle};
use aleph_primitives::{AlephSessionApi, KEY_TYPE};
use futures_timer::Delay;

use futures::channel::mpsc;
use log::{debug, error, info, trace, warn};

use codec::Encode;
use sc_client_api::{Backend, HeaderBackend};
use sc_network::ExHashT;
use sp_api::{BlockId, NumberFor};
use sp_consensus::SelectChain;
use sp_runtime::{
    traits::{Block, Header},
    SaturatedConversion,
};
use std::{
    collections::{HashMap, HashSet},
    default::Default,
    marker::PhantomData,
    sync::Arc,
    time::Duration,
};
use tokio::sync::Mutex;

mod task;
use task::{Handle, Task};
mod aggregator;
mod authority;
mod chain_tracker;
mod data_store;
mod member;
use authority::{
    SubtaskCommon as AuthoritySubtaskCommon, Subtasks as AuthoritySubtasks, Task as AuthorityTask,
};

type SplitData<B> = Split<AlephNetworkData<B>, RmcNetworkData<B>>;

pub struct AlephParams<B: Block, H: ExHashT, C, SC> {
    pub config: crate::AlephConfig<B, H, C, SC>,
}

impl<B: Block> Verifier<B> for AuthorityVerifier {
    fn verify(&self, justification: &AlephJustification, hash: B::Hash) -> bool {
        if !self.is_complete(&hash.encode()[..], &justification.signature) {
            warn!(target: "afa", "Bad justification for block hash #{:?} {:?}", hash, justification);
            return false;
        }
        true
    }
}

struct SessionInfoProviderImpl {
    session_authorities: Arc<Mutex<HashMap<SessionId, Vec<AuthorityId>>>>,
    session_period: SessionPeriod,
}

impl SessionInfoProviderImpl {
    fn new(
        session_authorities: Arc<Mutex<HashMap<SessionId, Vec<AuthorityId>>>>,
        session_period: SessionPeriod,
    ) -> Self {
        Self {
            session_authorities,
            session_period,
        }
    }
}

#[async_trait::async_trait]
impl<B: Block> SessionInfoProvider<B, AuthorityVerifier> for SessionInfoProviderImpl {
    async fn for_block_num(&self, number: NumberFor<B>) -> SessionInfo<B, AuthorityVerifier> {
        let current_session = session_id_from_block_num::<B>(number, self.session_period);
        let last_block_height = last_block_of_session::<B>(current_session, self.session_period);
        let verifier = self
            .session_authorities
            .lock()
            .await
            .get(&current_session)
            .map(|sa: &Vec<AuthorityId>| AuthorityVerifier::new(sa.to_vec()));

        SessionInfo {
            current_session,
            last_block_height,
            verifier,
        }
    }
}

///Max amount of tries we can not update a finalized block number before we will clear requests queue
const MAX_ATTEMPS: u32 = 5;

pub async fn run_consensus_party<B, H, C, BE, SC>(aleph_params: AlephParams<B, H, C, SC>)
where
    B: Block,
    H: ExHashT,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B>,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
{
    let AlephParams {
        config:
            crate::AlephConfig {
                network,
                client,
                select_chain,
                spawn_handle,
                keystore,
                justification_rx,
                metrics,
                session_period,
                millisecs_per_block,
                unit_creation_delay,
                ..
            },
    } = aleph_params;

    let session_authorities = Arc::new(Mutex::new(HashMap::new()));
    let block_requester = network.clone();

    let justification_handler_config = Default::default();

    let handler = JustificationHandler::new(
        SessionInfoProviderImpl::new(session_authorities.clone(), session_period),
        block_requester.clone(),
        client.clone(),
        AlephFinalizer::new(client.clone()),
        JustificationRequestSchedulerImpl::new(&session_period, &millisecs_per_block, MAX_ATTEMPS),
        metrics.clone(),
        justification_handler_config,
    );

    let authority_justification_tx =
        run_justification_handler(handler, &spawn_handle.clone().into(), justification_rx);

    // Prepare and start the network
    let (commands_for_network, commands_from_io) = mpsc::unbounded();
    let (messages_for_network, messages_from_user) = mpsc::unbounded();
    let (commands_for_service, commands_from_user) = mpsc::unbounded();
    let (messages_for_service, commands_from_manager) = mpsc::unbounded();
    let (messages_for_user, messages_from_network) = mpsc::unbounded();

    let connection_io = ConnectionIO::new(
        commands_for_network,
        messages_for_network,
        commands_from_user,
        commands_from_manager,
        messages_from_network,
    );
    let connection_manager = ConnectionManager::new(
        network.clone(),
        ConnectionManagerConfig::with_session_period(&session_period, &millisecs_per_block),
    );
    let session_manager = SessionManager::new(commands_for_service, messages_for_service);
    let network = NetworkService::new(
        network.clone(),
        spawn_handle.clone(),
        NetworkIO::new(messages_from_user, messages_for_user, commands_from_io),
    );

    let network_manager_task = async move {
        connection_io
            .run(connection_manager)
            .await
            .expect("Failed to run new network manager")
    };
    spawn_handle.spawn("aleph/network_manager", None, network_manager_task);
    let network_task = async move { network.run().await };
    spawn_handle.spawn("aleph/network", None, network_task);

    debug!(target: "afa", "Consensus network has started.");

    let party = ConsensusParty {
        session_manager,
        client,
        keystore,
        select_chain,
        block_requester,
        metrics,
        authority_justification_tx,
        session_authorities,
        session_period,
        spawn_handle: spawn_handle.into(),
        phantom: PhantomData,
        unit_creation_delay,
    };

    debug!(target: "afa", "Consensus party has started.");
    party.run().await;
    error!(target: "afa", "Consensus party has finished unexpectedly.");
}

async fn get_node_index(
    authorities: &[AuthorityId],
    keystore: Arc<dyn CryptoStore>,
) -> Option<NodeIndex> {
    let our_consensus_keys: HashSet<_> =
        keystore.keys(KEY_TYPE).await.unwrap().into_iter().collect();
    trace!(target: "afa", "Found {:?} consensus keys in our local keystore {:?}", our_consensus_keys.len(), our_consensus_keys);
    authorities
        .iter()
        .position(|pkey| our_consensus_keys.contains(&pkey.into()))
        .map(|id| id.into())
}

fn run_justification_handler<B, V, RB, C, S, SI, F>(
    handler: JustificationHandler<B, V, RB, C, S, SI, F>,
    spawn_handle: &crate::SpawnHandle,
    import_justification_rx: mpsc::UnboundedReceiver<JustificationNotification<B>>,
) -> mpsc::UnboundedSender<JustificationNotification<B>>
where
    C: HeaderBackend<B> + Send + Sync + 'static,
    B: Block,
    RB: RequestBlocks<B> + 'static,
    V: Verifier<B> + Send + 'static,
    S: JustificationRequestScheduler + Send + 'static,
    SI: SessionInfoProvider<B, V> + Send + Sync + 'static,
    F: BlockFinalizer<B> + Send + 'static,
{
    let (authority_justification_tx, authority_justification_rx) = mpsc::unbounded();

    debug!(target: "afa", "JustificationHandler started");
    spawn_handle.spawn("aleph/justification_handler", async move {
        handler
            .run(authority_justification_rx, import_justification_rx)
            .await;
    });

    authority_justification_tx
}

struct ConsensusParty<B, C, BE, SC, RB>
where
    B: Block,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B>,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
    RB: RequestBlocks<B> + 'static,
{
    session_manager: SessionManager<SplitData<B>>,
    session_authorities: Arc<Mutex<SessionMap>>,
    session_period: SessionPeriod,
    spawn_handle: crate::SpawnHandle,
    client: Arc<C>,
    select_chain: SC,
    keystore: Arc<dyn CryptoStore>,
    block_requester: RB,
    phantom: PhantomData<BE>,
    metrics: Option<Metrics<<B::Header as Header>::Hash>>,
    authority_justification_tx: mpsc::UnboundedSender<JustificationNotification<B>>,
    unit_creation_delay: UnitCreationDelay,
}

const SESSION_STATUS_CHECK_PERIOD: Duration = Duration::from_millis(1000);
const NEXT_SESSION_NETWORK_START_ATTEMPT_PERIOD: Duration = Duration::from_secs(30);

fn get_authorities_for_session<B, C>(
    runtime_api: sp_api::ApiRef<C::Api>,
    session_id: SessionId,
    first_block: NumberFor<B>,
) -> Vec<AuthorityId>
where
    B: Block,
    C: sp_api::ProvideRuntimeApi<B>,
    C::Api: aleph_primitives::AlephSessionApi<B>,
{
    if session_id == SessionId(0) {
        runtime_api
            .authorities(&BlockId::Number(<NumberFor<B>>::saturated_from(0u32)))
            .expect("Authorities for the session 0 must be available from the beginning")
    } else {
        runtime_api
            .next_session_authorities(&BlockId::Number(first_block))
            .unwrap_or_else(|_| {
                panic!(
                    "We didn't get the authorities for the session {:?}",
                    session_id
                )
            })
            .expect(
                "Authorities for next session must be available at first block of current session",
            )
    }
}

impl<B, C, BE, SC, RB> ConsensusParty<B, C, BE, SC, RB>
where
    B: Block,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B>,
    BE: Backend<B> + 'static,
    SC: SelectChain<B> + 'static,
    RB: RequestBlocks<B> + 'static,
{
    async fn spawn_authority_subtasks(
        &self,
        node_id: NodeIndex,
        multikeychain: KeyBox,
        data_network: SessionNetwork<SplitData<B>>,
        session_id: SessionId,
        authorities: Vec<AuthorityId>,
        exit_rx: futures::channel::oneshot::Receiver<()>,
    ) -> AuthoritySubtasks {
        debug!(target: "afa", "Authority task {:?}", session_id);
        let session_boundaries = SessionBoundaries::new(session_id, self.session_period);
        let (blocks_for_aggregator, blocks_from_interpreter) = mpsc::unbounded();

        let consensus_config = create_aleph_config(
            authorities.len(),
            node_id,
            session_id,
            self.unit_creation_delay,
        );

        let (chain_tracker, data_provider) = ChainTracker::new(
            self.select_chain.clone(),
            self.client.clone(),
            session_boundaries.clone(),
            self.metrics.clone(),
        );

        let ordered_data_interpreter = OrderedDataInterpreter::<B, C>::new(
            blocks_for_aggregator,
            self.client.clone(),
            session_boundaries.clone(),
        );

        let subtask_common = AuthoritySubtaskCommon {
            spawn_handle: self.spawn_handle.clone(),
            session_id: session_id.0,
        };
        let aggregator_io = aggregator::IO {
            blocks_from_interpreter,
            justifications_for_chain: self.authority_justification_tx.clone(),
        };

        let (unfiltered_aleph_network, rmc_network) = split(data_network);
        let (data_store, aleph_network) = DataStore::new(
            session_boundaries.clone(),
            self.client.clone(),
            self.block_requester.clone(),
            Default::default(),
            unfiltered_aleph_network,
        );

        AuthoritySubtasks::new(
            exit_rx,
            member::task(
                subtask_common.clone(),
                multikeychain.clone(),
                consensus_config,
                aleph_network.into(),
                data_provider,
                ordered_data_interpreter,
            ),
            aggregator::task(
                subtask_common.clone(),
                self.client.clone(),
                aggregator_io,
                session_boundaries,
                self.metrics.clone(),
                multikeychain,
                rmc_network,
            ),
            chain_tracker::task(subtask_common.clone(), chain_tracker),
            data_store::task(subtask_common, data_store),
        )
    }

    async fn spawn_authority_task(
        &self,
        session_id: SessionId,
        node_id: NodeIndex,
        authorities: Vec<AuthorityId>,
    ) -> AuthorityTask {
        let authority_verifier = AuthorityVerifier::new(authorities.clone());
        let authority_pen =
            AuthorityPen::new(authorities[node_id.0].clone(), self.keystore.clone())
                .await
                .expect("The keys should sign successfully");

        let keybox = KeyBox::new(node_id, authority_verifier.clone(), authority_pen.clone());

        let data_network = self
            .session_manager
            .start_validator_session(session_id, authority_verifier, node_id, authority_pen)
            .await
            .expect("Failed to start validator session!");

        let (exit, exit_rx) = futures::channel::oneshot::channel();
        let authority_subtasks = self
            .spawn_authority_subtasks(
                node_id,
                keybox,
                data_network,
                session_id,
                authorities,
                exit_rx,
            )
            .await;
        AuthorityTask::new(
            self.spawn_handle
                .spawn_essential("aleph/session_authority", async move {
                    if authority_subtasks.failed().await {
                        warn!(target: "aleph-party", "Authority subtasks failed.");
                    }
                }),
            node_id,
            exit,
        )
    }

    /// Returns authorities for a given session or None if the first block of this session is
    /// higher than the highest finalized block
    async fn updated_authorities_for_session(
        &mut self,
        session_id: SessionId,
    ) -> Option<Vec<AuthorityId>> {
        let last_finalized_number = self.client.info().finalized_number;
        let previous_session = match session_id {
            SessionId(0) => SessionId(0),
            SessionId(sid) => SessionId(sid - 1),
        };
        let first_block = first_block_of_session::<B>(previous_session, self.session_period);
        if last_finalized_number < first_block {
            return None;
        }

        Some(
            self.session_authorities
                .lock()
                .await
                .entry(session_id)
                .or_insert_with(|| {
                    get_authorities_for_session::<_, C>(
                        self.client.runtime_api(),
                        session_id,
                        first_block,
                    )
                })
                .clone(),
        )
    }

    async fn run_session(&mut self, session_id: SessionId) {
        let authorities = self
            .updated_authorities_for_session(session_id)
            .await
            .expect("We should know authorities for the session we are starting");
        let last_block = last_block_of_session::<B>(session_id, self.session_period);

        // Early skip attempt -- this will trigger during catching up (initial sync).
        if self.client.info().best_number >= last_block {
            // We need to give the JustificationHandler some time to pick up the keybox for the new session,
            // validate justifications and finalize blocks. We wait 2000ms in total, checking every 200ms
            // if the last block has been finalized.
            for attempt in 0..10 {
                // We don't wait before the first attempt.
                if attempt != 0 {
                    Delay::new(Duration::from_millis(200)).await;
                }
                let last_finalized_number = self.client.info().finalized_number;
                if last_finalized_number >= last_block {
                    debug!(target: "afa", "Skipping session {:?} early because block {:?} is already finalized", session_id, last_finalized_number);
                    return;
                }
            }
        }
        trace!(target: "afa", "Authorities for session {:?}: {:?}", session_id, authorities);
        let mut maybe_authority_task = if let Some(node_id) =
            get_node_index(&authorities, self.keystore.clone()).await
        {
            debug!(target: "afa", "Running session {:?} as authority id {:?}", session_id, node_id);
            Some(
                self.spawn_authority_task(session_id, node_id, authorities.clone())
                    .await,
            )
        } else {
            debug!(target: "afa", "Running session {:?} as non-authority", session_id);
            if let Err(e) = self
                .session_manager
                .start_nonvalidator_session(session_id, AuthorityVerifier::new(authorities.clone()))
            {
                warn!(target: "aleph-party", "Failed to start nonvalidator session{:?}:{:?}", session_id, e);
            }
            None
        };
        let mut check_session_status = Delay::new(SESSION_STATUS_CHECK_PERIOD);
        let mut start_next_session_network =
            Some(Delay::new(NEXT_SESSION_NETWORK_START_ATTEMPT_PERIOD));
        loop {
            tokio::select! {
                _ = &mut check_session_status => {
                    let last_finalized_number = self.client.info().finalized_number;
                    if last_finalized_number >= last_block {
                        debug!(target: "aleph-party", "Terminating session {:?}", session_id);
                        break;
                    }
                    check_session_status = Delay::new(SESSION_STATUS_CHECK_PERIOD);
                },
                Some(_) = async {
                    match &mut start_next_session_network {
                        Some(start_next_session_network) => {
                            start_next_session_network.await;
                            Some(())
                        },
                        None => None,
                    }
                } => {
                    let next_session_id = SessionId(session_id.0 + 1);
                    if let Some(next_session_authorities) =
                        self.updated_authorities_for_session(next_session_id).await
                    {
                        let authority_verifier = AuthorityVerifier::new(next_session_authorities.clone());
                        match get_node_index(&next_session_authorities, self.keystore.clone()).await {
                            Some(node_id) => {
                                let authority_pen = AuthorityPen::new(
                                    next_session_authorities[node_id.0].clone(),
                                    self.keystore.clone(),
                                )
                                .await
                                .expect("The keys should sign successfully");

                                if let Err(e) = self
                                    .session_manager
                                    .early_start_validator_session(
                                        next_session_id,
                                        authority_verifier,
                                        node_id,
                                        authority_pen,
                                    )
                                {
                                    warn!(target: "aleph-party", "Failed to early start validator session{:?}:{:?}", next_session_id, e);
                                }
                            }
                            None => {
                                if let Err(e) = self
                                    .session_manager
                                    .start_nonvalidator_session(next_session_id, authority_verifier)
                                {
                                    warn!(target: "aleph-party", "Failed to early start nonvalidator session{:?}:{:?}", next_session_id, e);
                                }
                            }
                        }
                        start_next_session_network = None;
                    } else {
                        start_next_session_network = Some(Delay::new(NEXT_SESSION_NETWORK_START_ATTEMPT_PERIOD));
                    }
                },
                Some(_) = async {
                    match maybe_authority_task.as_mut() {
                        Some(task) => Some(task.stopped().await),
                        None => None,
                    } } => {
                    warn!(target: "aleph-party", "Authority task ended prematurely, giving up for this session.");
                    maybe_authority_task = None;
                },
            }
        }
        if let Some(task) = maybe_authority_task {
            debug!(target: "aleph-party", "Stopping the authority task.");
            task.stop().await;
        }
        if let Err(e) = self.session_manager.stop_session(session_id) {
            warn!(target: "aleph-party", "Session Manager failed to stop in session {:?}: {:?}", session_id, e)
        }
    }

    async fn prune_session_data(&self, prune_below: SessionId) {
        // In this method we make sure that the amount of data we keep in RAM in finality-aleph
        // does not grow with the size of the blockchain.
        debug!(target: "afa", "Pruning session data below {:?}.", prune_below);
        self.session_authorities
            .lock()
            .await
            .retain(|&s, _| s >= prune_below);
    }

    async fn run(mut self) {
        let last_finalized_number = self.client.info().finalized_number;
        let starting_session =
            session_id_from_block_num::<B>(last_finalized_number, self.session_period);
        for curr_id in starting_session.0.. {
            info!(target: "afa", "Running session {:?}.", curr_id);
            self.run_session(SessionId(curr_id)).await;
            if curr_id >= 10 && curr_id % 10 == 0 {
                self.prune_session_data(SessionId(curr_id - 10)).await;
            }
        }
    }
}

pub(crate) fn create_aleph_config(
    n_members: usize,
    node_id: NodeIndex,
    session_id: SessionId,
    unit_creation_delay: UnitCreationDelay,
) -> aleph_bft::Config {
    let mut consensus_config = default_aleph_config(n_members.into(), node_id, session_id.0 as u64);
    consensus_config.max_round = 7000;
    let unit_creation_delay = Arc::new(move |t| {
        if t == 0 {
            Duration::from_millis(2000)
        } else {
            exponential_slowdown(t, unit_creation_delay.0 as f64, 5000, 1.005)
        }
    });
    let unit_broadcast_delay = Arc::new(|t| exponential_slowdown(t, 4000., 0, 2.));
    let delay_config = DelayConfig {
        tick_interval: Duration::from_millis(100),
        requests_interval: Duration::from_millis(3000),
        unit_broadcast_delay,
        unit_creation_delay,
    };
    consensus_config.delay_config = delay_config;
    consensus_config
}

pub fn exponential_slowdown(
    t: usize,
    base_delay: f64,
    start_exp_delay: usize,
    exp_base: f64,
) -> Duration {
    // This gives:
    // base_delay, for t <= start_exp_delay,
    // base_delay * exp_base^(t - start_exp_delay), for t > start_exp_delay.
    let delay = if t < start_exp_delay {
        base_delay
    } else {
        let power = t - start_exp_delay;
        base_delay * exp_base.powf(power as f64)
    };
    let delay = delay.round() as u64;
    // the above will make it u64::MAX if it exceeds u64
    Duration::from_millis(delay)
}

// TODO: :(
#[cfg(test)]
mod tests {}
