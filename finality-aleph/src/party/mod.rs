use std::{default::Default, marker::PhantomData, path::PathBuf, time::Duration};

use futures_timer::Delay;
use log::{debug, error, info, trace, warn};
use tokio::{task::spawn_blocking, time::sleep};

use crate::{
    party::{
        manager::{Handle, SubtaskCommon as AuthoritySubtaskCommon, Task},
        traits::{Block, ChainState, NodeSessionManager, SessionInfo, SyncState},
    },
    session_map::ReadOnlySessionMap,
    SessionId,
};

pub(crate) mod backup;
pub mod impls;
pub mod manager;
pub mod traits;

#[cfg(test)]
mod mocks;

pub(crate) struct ConsensusPartyParams<B: Block, ST, CS, NSM, SI> {
    pub session_authorities: ReadOnlySessionMap,
    pub chain_state: CS,
    pub sync_state: ST,
    pub backup_saving_path: Option<PathBuf>,
    pub session_manager: NSM,
    pub session_info: SI,
    pub _phantom: PhantomData<B>,
}

pub(crate) struct ConsensusParty<B, ST, CS, NSM, SI>
where
    B: Block,
    ST: SyncState<B>,
    CS: ChainState<B>,
    NSM: NodeSessionManager,
    SI: SessionInfo<B>,
{
    session_authorities: ReadOnlySessionMap,
    chain_state: CS,
    sync_state: ST,
    backup_saving_path: Option<PathBuf>,
    session_manager: NSM,
    session_info: SI,
    _phantom: PhantomData<B>,
}

const SESSION_STATUS_CHECK_PERIOD: Duration = Duration::from_millis(1000);

impl<B, ST, CS, NSM, SI> ConsensusParty<B, ST, CS, NSM, SI>
where
    B: Block,
    ST: SyncState<B>,
    CS: ChainState<B>,
    NSM: NodeSessionManager,
    SI: SessionInfo<B>,
{
    pub(crate) fn new(params: ConsensusPartyParams<B, ST, CS, NSM, SI>) -> Self {
        let ConsensusPartyParams {
            session_authorities,
            sync_state,
            backup_saving_path,
            chain_state,
            session_manager,
            session_info,
            ..
        } = params;
        Self {
            sync_state,
            session_authorities,
            backup_saving_path,
            chain_state,
            session_manager,
            session_info,
            _phantom: PhantomData,
        }
    }

    async fn run_session(&mut self, session_id: SessionId) {
        let last_block = self.session_info.last_block_of_session(session_id);
        if let Some(previous_session_id) = session_id.0.checked_sub(1) {
            let backup_saving_path = self.backup_saving_path.clone();
            spawn_blocking(move || backup::remove(backup_saving_path, previous_session_id));
        }

        // Early skip attempt -- this will trigger during catching up (initial sync).
        if self.chain_state.best_block_number() >= last_block {
            // We need to give the JustificationHandler some time to pick up the keychain for the new session,
            // validate justifications and finalize blocks. We wait 2000ms in total, checking every 200ms
            // if the last block has been finalized.
            for attempt in 0..10 {
                // We don't wait before the first attempt.
                if attempt != 0 {
                    Delay::new(Duration::from_millis(200)).await;
                }
                let last_finalized_number = self.chain_state.finalized_number();
                if last_finalized_number >= last_block {
                    debug!(target: "aleph-party", "Skipping session {:?} early because block {:?} is already finalized", session_id, last_finalized_number);
                    return;
                }
            }
        }

        // We need to wait until session authority data is available for current session.
        // This should only be needed for the first ever session as all other session are known
        // at least one session earlier.
        let authority_data = match self
            .session_authorities
            .subscribe_to_insertion(session_id)
            .await
            .await
        {
            Err(e) => panic!(
                "Error while receiving the notification about current session {:?}",
                e
            ),
            Ok(authority_data) => authority_data,
        };
        let authorities = authority_data.authorities();

        trace!(target: "aleph-party", "Authority data for session {:?}: {:?}", session_id, authorities);
        let mut maybe_authority_task = if let Some(node_id) =
            self.session_manager.node_idx(authorities).await
        {
            match backup::rotate(self.backup_saving_path.clone(), session_id.0) {
                Ok(backup) => {
                    debug!(target: "aleph-party", "Running session {:?} as authority id {:?}", session_id, node_id);
                    Some(
                        self.session_manager
                            .spawn_authority_task_for_session(
                                session_id,
                                node_id,
                                backup,
                                authorities,
                            )
                            .await,
                    )
                }
                Err(err) => {
                    error!(
                        target: "AlephBFT-member",
                        "Error setting up backup saving for session {:?}. Not running the session: {}",
                        session_id, err
                    );
                    return;
                }
            }
        } else {
            debug!(target: "aleph-party", "Running session {:?} as non-authority", session_id);
            if let Err(e) = self
                .session_manager
                .start_nonvalidator_session(session_id, authorities)
            {
                warn!(target: "aleph-party", "Failed to start nonvalidator session{:?}:{:?}", session_id, e);
            }
            None
        };
        let mut check_session_status = Delay::new(SESSION_STATUS_CHECK_PERIOD);
        let next_session_id = SessionId(session_id.0 + 1);
        let mut start_next_session_network = Some(
            self.session_authorities
                .subscribe_to_insertion(next_session_id)
                .await,
        );
        loop {
            tokio::select! {
                _ = &mut check_session_status => {
                    let last_finalized_number = self.chain_state.finalized_number();
                    if last_finalized_number >= last_block {
                        debug!(target: "aleph-party", "Terminating session {:?}", session_id);
                        break;
                    }
                    check_session_status = Delay::new(SESSION_STATUS_CHECK_PERIOD);
                },
                Some(next_session_authority_data) = async {
                    match &mut start_next_session_network {
                        Some(notification) => {
                            match notification.await {
                                Err(e) => {
                                    warn!(target: "aleph-party", "Error with subscription {:?}", e);
                                    start_next_session_network = Some(self.session_authorities.subscribe_to_insertion(next_session_id).await);
                                    None
                                },
                                Ok(next_session_authority_data) => {
                                    Some(next_session_authority_data)
                                }
                            }
                        },
                        None => None,
                    }
                } => {
                    let next_session_authorities = next_session_authority_data.authorities();
                    match self.session_manager.node_idx(next_session_authorities).await {
                         Some(_) => if let Err(e) = self
                                .session_manager
                                .early_start_validator_session(
                                    next_session_id,
                                    next_session_authorities,
                                ).await
                            {
                                warn!(target: "aleph-party", "Failed to early start validator session{:?}:{:?}", next_session_id, e);
                            }
                        None => {
                            if let Err(e) = self
                                .session_manager
                                .start_nonvalidator_session(next_session_id, next_session_authorities)
                            {
                                warn!(target: "aleph-party", "Failed to early start nonvalidator session{:?}:{:?}", next_session_id, e);
                            }
                        }
                    }
                    start_next_session_network = None;
                },
                Some(_) = async {
                    match maybe_authority_task.as_mut() {
                        Some(task) => Some(task.stopped().await),
                        None => None,
                    }
                } => {
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

    pub async fn run(mut self) {
        let starting_session = self.catch_up().await;
        for curr_id in starting_session.0.. {
            println!("Running session {:?}.", curr_id);
            info!(target: "aleph-party", "Running session {:?}.", curr_id);
            self.run_session(SessionId(curr_id)).await;
        }
    }

    async fn catch_up(&mut self) -> SessionId {
        let mut finalized_number = self.chain_state.finalized_number();
        let mut previous_finalized_number = None;
        while self.sync_state.is_major_syncing()
            && Some(finalized_number) != previous_finalized_number
        {
            sleep(Duration::from_millis(500)).await;
            previous_finalized_number = Some(finalized_number);
            finalized_number = self.chain_state.finalized_number();
        }
        self.session_info
            .session_id_from_block_num(finalized_number)
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, sync::Arc, time::Duration};

    use aleph_primitives::{AuthorityId, SessionAuthorityData};
    use sp_runtime::testing::UintAuthorityId;
    use tokio::time::sleep;

    use crate::{
        party::{
            mocks::{
                MockChainState, MockNodeSessionManager, MockSessionInfo, MockSyncState, SimpleBlock,
            },
            ConsensusParty, ConsensusPartyParams, SESSION_STATUS_CHECK_PERIOD,
        },
        session_map::SharedSessionMap,
        SessionId,
    };

    const SESSION_PERIOD: u32 = 30;

    #[derive(Debug)]
    struct MockController {
        pub shared_session_map: SharedSessionMap,
        pub _sync_state_mock: Arc<MockSyncState>,
        pub chain_state_mock: Arc<MockChainState>,
        pub node_session_manager: Arc<MockNodeSessionManager>,
    }

    fn create_mocked_consensus_party() -> (
        ConsensusParty<
            SimpleBlock,
            Arc<MockSyncState>,
            Arc<MockChainState>,
            Arc<MockNodeSessionManager>,
            MockSessionInfo,
        >,
        MockController,
    ) {
        let shared_map = SharedSessionMap::new();
        let readonly_session_authorities = shared_map.read_only();

        let chain_state = Arc::new(MockChainState::new());
        let sync_state = Arc::new(MockSyncState::new());
        let session_manager = Arc::new(MockNodeSessionManager::new());
        let session_info = MockSessionInfo::new(SESSION_PERIOD);

        let controller = MockController {
            shared_session_map: shared_map,
            _sync_state_mock: sync_state.clone(),
            chain_state_mock: chain_state.clone(),
            node_session_manager: session_manager.clone(),
        };

        let params = ConsensusPartyParams {
            session_authorities: readonly_session_authorities,
            chain_state,
            sync_state,
            backup_saving_path: None,
            session_manager,
            session_info,
            _phantom: Default::default(),
        };

        (ConsensusParty::new(params), controller)
    }

    fn simulate_n_blocks(start: u32, n: u32, controller: &MockController) {
        for i in start..start + n {
            controller.chain_state_mock.set_best_block(i);
            controller.chain_state_mock.set_finalized_block(i);
        }
    }

    async fn set_authorities_for_sessions(
        controller: &mut MockController,
        sessions: Vec<(SessionId, Vec<AuthorityId>)>,
    ) {
        for (session, authorities) in sessions {
            controller
                .shared_session_map
                .update(session, SessionAuthorityData::new(authorities, None))
                .await;
        }
    }

    fn assert_session_states(
        controller: &MockController,
        validator_started: Vec<SessionId>,
        early_started: Vec<SessionId>,
        stopped: Vec<SessionId>,
        non_validator_started: Vec<SessionId>,
    ) {
        assert_eq!(
            *controller
                .node_session_manager
                .validator_session_started
                .lock()
                .unwrap(),
            HashSet::from_iter(validator_started),
            "`validator_session_started` mismatch"
        );

        assert_eq!(
            *controller
                .node_session_manager
                .session_early_started
                .lock()
                .unwrap(),
            HashSet::from_iter(early_started),
            "`session_early_started` mismatch"
        );

        assert_eq!(
            *controller
                .node_session_manager
                .session_stopped
                .lock()
                .unwrap(),
            HashSet::from_iter(stopped),
            "`session_stopped` mismatch"
        );

        assert_eq!(
            *controller
                .node_session_manager
                .nonvalidator_session_started
                .lock()
                .unwrap(),
            HashSet::from_iter(non_validator_started),
            "`nonvalidator_session_started` mismatch"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn party_starts_session_for_node_in_authorities() {
        let (party, mut controller) = create_mocked_consensus_party();

        controller
            .node_session_manager
            .set_node_id(Some(UintAuthorityId(0).to_public_key()));

        let _party_handle = tokio::spawn(party.run());

        let authorities: Vec<_> = (0..10)
            .map(|id| UintAuthorityId(id).to_public_key())
            .collect();
        set_authorities_for_sessions(
            &mut controller,
            vec![
                (SessionId(0), authorities.clone()),
                (SessionId(1), authorities),
            ],
        )
        .await;

        sleep(Duration::from_millis(100)).await;
        simulate_n_blocks(0, SESSION_PERIOD, &controller);
        sleep(Duration::from_millis(
            SESSION_STATUS_CHECK_PERIOD.as_millis() as u64 + 100,
        ))
        .await;

        assert_session_states(
            &controller,
            vec![SessionId(0), SessionId(1)],
            vec![SessionId(1)],
            vec![SessionId(0)],
            vec![],
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn party_run_3_authorities_sessions() {
        let (party, mut controller) = create_mocked_consensus_party();

        controller
            .node_session_manager
            .set_node_id(Some(UintAuthorityId(0).to_public_key()));

        let _party_handle = tokio::spawn(party.run());

        let authorities: Vec<_> = (0..10)
            .map(|id| UintAuthorityId(id).to_public_key())
            .collect();
        set_authorities_for_sessions(
            &mut controller,
            vec![
                (SessionId(0), authorities.clone()),
                (SessionId(1), authorities.clone()),
                (SessionId(2), authorities.clone()),
                (SessionId(3), authorities),
            ],
        )
        .await;

        sleep(Duration::from_millis(100)).await;

        for i in 0..3 {
            simulate_n_blocks(SESSION_PERIOD * i, SESSION_PERIOD, &controller);
            sleep(Duration::from_millis(
                SESSION_STATUS_CHECK_PERIOD.as_millis() as u64 + 100,
            ))
            .await;
        }

        assert_session_states(
            &controller,
            vec![SessionId(0), SessionId(1), SessionId(2), SessionId(3)],
            vec![SessionId(1), SessionId(2), SessionId(3)],
            vec![SessionId(0), SessionId(1), SessionId(2)],
            vec![],
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn party_run_3_non_authorities_sessions() {
        let (party, mut controller) = create_mocked_consensus_party();

        controller.node_session_manager.set_node_id(None);

        let _party_handle = tokio::spawn(party.run());

        let authorities: Vec<_> = (0..10)
            .map(|id| UintAuthorityId(id).to_public_key())
            .collect();
        set_authorities_for_sessions(
            &mut controller,
            vec![
                (SessionId(0), authorities.clone()),
                (SessionId(1), authorities.clone()),
                (SessionId(2), authorities.clone()),
                (SessionId(3), authorities),
            ],
        )
        .await;

        sleep(Duration::from_millis(100)).await;

        for i in 0..3 {
            simulate_n_blocks(SESSION_PERIOD * i, SESSION_PERIOD, &controller);
            sleep(Duration::from_millis(
                SESSION_STATUS_CHECK_PERIOD.as_millis() as u64 + 100,
            ))
            .await;
        }

        assert_session_states(
            &controller,
            vec![],
            vec![],
            vec![SessionId(0), SessionId(1), SessionId(2)],
            vec![SessionId(0), SessionId(1), SessionId(2), SessionId(3)],
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn party_early_skips_past_sessions() {
        let (party, mut controller) = create_mocked_consensus_party();

        controller
            .node_session_manager
            .set_node_id(Some(UintAuthorityId(0).to_public_key()));

        controller
            .chain_state_mock
            .set_finalized_block(SESSION_PERIOD * 2);
        controller
            .chain_state_mock
            .set_best_block(SESSION_PERIOD * 2);

        let _party_handle = tokio::spawn(party.run());

        let authorities: Vec<_> = (0..10)
            .map(|id| UintAuthorityId(id).to_public_key())
            .collect();
        set_authorities_for_sessions(
            &mut controller,
            vec![
                (SessionId(0), authorities.clone()),
                (SessionId(1), authorities.clone()),
                (SessionId(2), authorities.clone()),
                (SessionId(3), authorities),
            ],
        )
        .await;

        sleep(Duration::from_millis(
            SESSION_STATUS_CHECK_PERIOD.as_millis() as u64 + 100,
        ))
        .await;

        assert_session_states(
            &controller,
            vec![SessionId(2)],
            vec![SessionId(3)],
            vec![],
            vec![],
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn party_dont_start_session_for_node_non_in_authorities() {
        let (party, mut controller) = create_mocked_consensus_party();

        controller
            .node_session_manager
            .set_node_id(Some(UintAuthorityId(0).to_public_key()));

        let _party_handle = tokio::spawn(party.run());

        let authorities: Vec<_> = (0..10)
            .map(|id| UintAuthorityId(id).to_public_key())
            .collect();
        set_authorities_for_sessions(
            &mut controller,
            vec![
                (SessionId(0), authorities.clone()),
                (SessionId(1), authorities[1..].to_vec()),
            ],
        )
        .await;

        sleep(Duration::from_millis(100)).await;
        simulate_n_blocks(0, SESSION_PERIOD + 1, &controller);
        sleep(Duration::from_millis(
            SESSION_STATUS_CHECK_PERIOD.as_millis() as u64 + 100,
        ))
        .await;

        assert_session_states(
            &controller,
            vec![SessionId(0)],
            vec![],
            vec![SessionId(0)],
            vec![SessionId(1)],
        );
    }
}
