use std::{default::Default, path::PathBuf, time::Duration};

use futures_timer::Delay;
use log::{debug, error, info, trace, warn};
use tokio::{task::spawn_blocking, time::sleep};

use crate::{
    party::{
        manager::{Handle, Task, TaskCommon as AuthoritySubtaskCommon},
        traits::{ChainState, NodeSessionManager},
    },
    session::SessionBoundaryInfo,
    session_map::ReadOnlySessionMap,
    SessionId, SyncOracle,
};

pub(crate) mod backup;
pub mod impls;
pub mod manager;
pub mod traits;

#[cfg(test)]
mod mocks;

pub(crate) struct ConsensusPartyParams<CS, NSM> {
    pub session_authorities: ReadOnlySessionMap,
    pub chain_state: CS,
    pub sync_oracle: SyncOracle,
    pub backup_saving_path: Option<PathBuf>,
    pub session_manager: NSM,
    pub session_info: SessionBoundaryInfo,
}

pub(crate) struct ConsensusParty<CS, NSM>
where
    CS: ChainState,
    NSM: NodeSessionManager,
{
    session_authorities: ReadOnlySessionMap,
    chain_state: CS,
    sync_oracle: SyncOracle,
    backup_saving_path: Option<PathBuf>,
    session_manager: NSM,
    session_info: SessionBoundaryInfo,
}

const SESSION_STATUS_CHECK_PERIOD: Duration = Duration::from_millis(1000);

impl<CS, NSM> ConsensusParty<CS, NSM>
where
    CS: ChainState,
    NSM: NodeSessionManager,
{
    pub(crate) fn new(params: ConsensusPartyParams<CS, NSM>) -> Self {
        let ConsensusPartyParams {
            session_authorities,
            sync_oracle,
            backup_saving_path,
            chain_state,
            session_manager,
            session_info,
            ..
        } = params;
        Self {
            sync_oracle,
            session_authorities,
            backup_saving_path,
            chain_state,
            session_manager,
            session_info,
        }
    }

    async fn run_session(&mut self, session_id: SessionId) {
        let last_block = self.session_info.last_block_of_session(session_id);
        if session_id.0.checked_sub(1).is_some() {
            let backup_saving_path = self.backup_saving_path.clone();
            spawn_blocking(move || {
                if let Err(e) = backup::remove_old_backups(backup_saving_path, session_id.0) {
                    warn!(target: "aleph-party", "Error when clearing old backups: {}", e);
                }
            });
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
            Err(e) => panic!("Error while receiving the notification about current session {e:?}"),
            Ok(authority_data) => authority_data,
        };
        let authorities = authority_data.authorities();

        trace!(target: "aleph-party", "Authority data for session {:?}: {:?}", session_id, authorities);
        let mut maybe_authority_task = if let Some(node_id) =
            self.session_manager.node_idx(authorities)
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
                warn!(target: "aleph-party", "Failed to start nonvalidator session{:?}: {}", session_id, e);
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
                    match self.session_manager.node_idx(next_session_authorities) {
                         Some(next_session_node_id) => if let Err(e) = self
                                .session_manager
                                .early_start_validator_session(
                                    next_session_id,
                                    next_session_node_id,
                                    next_session_authorities,
                                )
                            {
                                warn!(target: "aleph-party", "Failed to early start validator session{:?}: {}", next_session_id, e);
                            }
                        None => {
                            if let Err(e) = self
                                .session_manager
                                .start_nonvalidator_session(next_session_id, next_session_authorities)
                            {
                                warn!(target: "aleph-party", "Failed to early start nonvalidator session{:?}: {}", next_session_id, e);
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
            if task.stop().await.is_err() {
                warn!(target: "aleph-party", "Authority task did not stop silently");
            }
        }
        if let Err(e) = self.session_manager.stop_session(session_id) {
            warn!(target: "aleph-party", "Session Manager failed to stop in session {:?}: {}", session_id, e)
        }
    }

    pub async fn run(mut self) {
        let starting_session = self.catch_up().await;
        for curr_id in starting_session.0.. {
            info!(target: "aleph-party", "Running session {:?}.", curr_id);
            self.run_session(SessionId(curr_id)).await;
        }
    }

    async fn catch_up(&mut self) -> SessionId {
        let mut finalized_number = self.chain_state.finalized_number();
        let mut previous_finalized_number = None;
        while self.sync_oracle.major_sync() && Some(finalized_number) != previous_finalized_number {
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
    use std::{
        collections::{HashMap, HashSet},
        sync::Arc,
        time::Duration,
    };

    use sp_runtime::testing::UintAuthorityId;
    use tokio::{task::JoinHandle, time::sleep};

    use crate::{
        aleph_primitives::{AuthorityId, SessionAuthorityData},
        party::{
            mocks::{MockChainState, MockNodeSessionManager},
            ConsensusParty, ConsensusPartyParams, SESSION_STATUS_CHECK_PERIOD,
        },
        session::SessionBoundaryInfo,
        session_map::SharedSessionMap,
        SessionId, SessionPeriod, SyncOracle,
    };

    type Party = ConsensusParty<Arc<MockChainState>, Arc<MockNodeSessionManager>>;

    struct PartyState {
        validator_started: Vec<SessionId>,
        early_started: Vec<SessionId>,
        stopped: Vec<SessionId>,
        non_validator_started: Vec<SessionId>,
    }

    #[derive(Default)]
    struct BlockEvents {
        session_authorities: Option<(SessionId, Vec<AuthorityId>)>,
        id: Option<Option<AuthorityId>>,
        state_to_assert: Option<PartyState>,
    }

    struct PartyTest {
        current_block: u32,
        controller: MockController,
        block_events: HashMap<u32, BlockEvents>,
        handle: Option<JoinHandle<()>>,
    }

    impl PartyTest {
        fn new(session_period: SessionPeriod) -> (Self, Party) {
            let (party, controller) = create_mocked_consensus_party(session_period);

            (
                Self {
                    current_block: 0,
                    controller,
                    block_events: Default::default(),
                    handle: None,
                },
                party,
            )
        }

        fn run_party(mut self, party: Party) -> Self {
            let party_handle = tokio::spawn(party.run());
            self.handle = Some(party_handle);

            self
        }

        fn assert_state(&self, expected_state: PartyState, block: u32) {
            let PartyState {
                validator_started,
                early_started,
                stopped,
                non_validator_started,
            } = expected_state;
            assert_eq!(
                *self
                    .controller
                    .node_session_manager
                    .validator_session_started
                    .lock()
                    .unwrap(),
                HashSet::from_iter(validator_started),
                "`validator_session_started` mismatch at block #{block}"
            );

            assert_eq!(
                *self
                    .controller
                    .node_session_manager
                    .session_early_started
                    .lock()
                    .unwrap(),
                HashSet::from_iter(early_started),
                "`session_early_started` mismatch at block #{block}"
            );

            assert_eq!(
                *self
                    .controller
                    .node_session_manager
                    .session_stopped
                    .lock()
                    .unwrap(),
                HashSet::from_iter(stopped),
                "`session_stopped` mismatch at block #{block}"
            );

            assert_eq!(
                *self
                    .controller
                    .node_session_manager
                    .nonvalidator_session_started
                    .lock()
                    .unwrap(),
                HashSet::from_iter(non_validator_started),
                "`nonvalidator_session_started` mismatch at block #{block}"
            );
        }

        async fn run_for_n_blocks(mut self, n: u32) -> Self {
            for i in self.current_block..self.current_block + n {
                self.controller.chain_state_mock.set_best_block(i);
                self.controller.chain_state_mock.set_finalized_block(i);

                if let Some(events) = self.block_events.remove(&i) {
                    self.handle_events(events, i).await;
                }
            }

            self.current_block += n;

            self
        }

        async fn handle_events(&mut self, events: BlockEvents, block: u32) {
            let BlockEvents {
                session_authorities,
                id,
                state_to_assert,
            } = events;

            if let Some(expected_state) = state_to_assert {
                // sleep to make sure party catch all events
                sleep(Duration::from_millis(
                    SESSION_STATUS_CHECK_PERIOD.as_millis() as u64 + 100,
                ))
                .await;
                self.assert_state(expected_state, block);
            }

            if let Some((session, authorities)) = session_authorities {
                self.controller
                    .shared_session_map
                    .update(session, SessionAuthorityData::new(authorities, None))
                    .await;
            }

            if let Some(id) = id {
                self.controller.node_session_manager.set_node_id(id)
            }
        }

        fn set_authorities_for_session_at_block(
            mut self,
            block: u32,
            authorities: Vec<AuthorityId>,
            session: SessionId,
        ) -> Self {
            let events = self.block_events.entry(block).or_default();
            events.session_authorities = Some((session, authorities));

            self
        }

        fn set_node_id_for_session_at_block(mut self, block: u32, id: Option<AuthorityId>) -> Self {
            let events = self.block_events.entry(block).or_default();
            events.id = Some(id);

            self
        }

        async fn set_best_and_finalized_block(
            mut self,
            best_block: u32,
            finalized_block: u32,
        ) -> Self {
            self.controller.chain_state_mock.set_best_block(best_block);
            self.controller
                .chain_state_mock
                .set_finalized_block(finalized_block);

            self.current_block = best_block + 1;

            self
        }

        fn expect_session_states_at_block(
            mut self,
            block: u32,
            expected_state: PartyState,
        ) -> Self {
            let events = self.block_events.entry(block).or_default();
            events.state_to_assert = Some(expected_state);

            self
        }

        async fn set_now(
            mut self,
            session_authorities: Option<(SessionId, Vec<AuthorityId>)>,
            id: Option<Option<AuthorityId>>,
        ) -> Self {
            if let Some((session, authorities)) = session_authorities {
                self.controller
                    .shared_session_map
                    .update(session, SessionAuthorityData::new(authorities, None))
                    .await;
            }

            if let Some(id) = id {
                self.controller.node_session_manager.set_node_id(id)
            }

            self
        }
    }

    const SESSION_PERIOD: u32 = 30;

    #[derive(Debug)]
    struct MockController {
        pub shared_session_map: SharedSessionMap,
        pub chain_state_mock: Arc<MockChainState>,
        pub node_session_manager: Arc<MockNodeSessionManager>,
    }

    #[allow(clippy::type_complexity)]
    fn create_mocked_consensus_party(
        session_period: SessionPeriod,
    ) -> (
        ConsensusParty<Arc<MockChainState>, Arc<MockNodeSessionManager>>,
        MockController,
    ) {
        let shared_map = SharedSessionMap::new();
        let readonly_session_authorities = shared_map.read_only();

        let chain_state = Arc::new(MockChainState::new());
        let sync_oracle = SyncOracle::new();
        let session_manager = Arc::new(MockNodeSessionManager::new());
        let session_info = SessionBoundaryInfo::new(session_period);

        let controller = MockController {
            shared_session_map: shared_map,
            chain_state_mock: chain_state.clone(),
            node_session_manager: session_manager.clone(),
        };

        let params = ConsensusPartyParams {
            session_authorities: readonly_session_authorities,
            chain_state,
            sync_oracle,
            backup_saving_path: None,
            session_manager,
            session_info,
        };

        (ConsensusParty::new(params), controller)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn party_starts_session_for_node_in_authorities() {
        let (test, party) = PartyTest::new(SessionPeriod(SESSION_PERIOD));

        let authorities: Vec<_> = (0..10)
            .map(|id| UintAuthorityId(id).to_public_key())
            .collect();

        let state_1 = PartyState {
            validator_started: vec![SessionId(0)],
            early_started: vec![SessionId(1)],
            stopped: vec![],
            non_validator_started: vec![],
        };

        let state_2 = PartyState {
            validator_started: vec![SessionId(0), SessionId(1)],
            early_started: vec![SessionId(1)],
            stopped: vec![SessionId(0)],
            non_validator_started: vec![],
        };

        test.set_authorities_for_session_at_block(0, authorities.clone(), SessionId(0))
            .set_authorities_for_session_at_block(25, authorities, SessionId(1))
            .set_node_id_for_session_at_block(0, Some(UintAuthorityId(0).to_public_key()))
            .expect_session_states_at_block(28, state_1)
            .expect_session_states_at_block(29, state_2)
            .run_party(party)
            .run_for_n_blocks(SESSION_PERIOD)
            .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn party_run_3_authorities_sessions() {
        let (test, party) = PartyTest::new(SessionPeriod(SESSION_PERIOD));

        let authorities: Vec<_> = (0..10)
            .map(|id| UintAuthorityId(id).to_public_key())
            .collect();

        let state_1 = PartyState {
            validator_started: vec![SessionId(0)],
            early_started: vec![SessionId(1)],
            stopped: vec![],
            non_validator_started: vec![],
        };

        let state_2 = PartyState {
            validator_started: vec![SessionId(0), SessionId(1)],
            early_started: vec![SessionId(1)],
            stopped: vec![SessionId(0)],
            non_validator_started: vec![],
        };

        let state_3 = PartyState {
            validator_started: vec![SessionId(0), SessionId(1), SessionId(2)],
            early_started: vec![SessionId(1), SessionId(2)],
            stopped: vec![SessionId(0), SessionId(1)],
            non_validator_started: vec![],
        };

        let state_4 = PartyState {
            validator_started: vec![SessionId(0), SessionId(1), SessionId(2), SessionId(3)],
            early_started: vec![SessionId(1), SessionId(2), SessionId(3)],
            stopped: vec![SessionId(0), SessionId(1), SessionId(2)],
            non_validator_started: vec![],
        };

        test.set_authorities_for_session_at_block(0, authorities.clone(), SessionId(0))
            .set_authorities_for_session_at_block(25, authorities.clone(), SessionId(1))
            .set_authorities_for_session_at_block(55, authorities.clone(), SessionId(2))
            .set_authorities_for_session_at_block(85, authorities, SessionId(3))
            .set_node_id_for_session_at_block(0, Some(UintAuthorityId(0).to_public_key()))
            .expect_session_states_at_block(28, state_1)
            .expect_session_states_at_block(29, state_2)
            .expect_session_states_at_block(59, state_3)
            .expect_session_states_at_block(89, state_4)
            .run_party(party)
            .run_for_n_blocks(3 * SESSION_PERIOD)
            .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn party_run_3_non_authorities_sessions() {
        let (test, party) = PartyTest::new(SessionPeriod(SESSION_PERIOD));

        let authorities: Vec<_> = (0..10)
            .map(|id| UintAuthorityId(id).to_public_key())
            .collect();

        let state_1 = PartyState {
            non_validator_started: vec![SessionId(0)],
            early_started: vec![],
            stopped: vec![],
            validator_started: vec![],
        };

        let state_2 = PartyState {
            non_validator_started: vec![SessionId(0), SessionId(1)],
            early_started: vec![],
            stopped: vec![SessionId(0)],
            validator_started: vec![],
        };

        let state_3 = PartyState {
            non_validator_started: vec![SessionId(0), SessionId(1), SessionId(2)],
            early_started: vec![],
            stopped: vec![SessionId(0), SessionId(1)],
            validator_started: vec![],
        };

        let state_4 = PartyState {
            non_validator_started: vec![SessionId(0), SessionId(1), SessionId(2), SessionId(3)],
            early_started: vec![],
            stopped: vec![SessionId(0), SessionId(1), SessionId(2)],
            validator_started: vec![],
        };

        test.set_authorities_for_session_at_block(0, authorities.clone(), SessionId(0))
            .set_authorities_for_session_at_block(25, authorities.clone(), SessionId(1))
            .set_authorities_for_session_at_block(55, authorities.clone(), SessionId(2))
            .set_authorities_for_session_at_block(85, authorities, SessionId(3))
            .set_node_id_for_session_at_block(0, None)
            .expect_session_states_at_block(24, state_1)
            .expect_session_states_at_block(29, state_2)
            .expect_session_states_at_block(59, state_3)
            .expect_session_states_at_block(89, state_4)
            .run_party(party)
            .run_for_n_blocks(3 * SESSION_PERIOD)
            .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn party_early_skips_past_sessions() {
        let (test, party) = PartyTest::new(SessionPeriod(SESSION_PERIOD));

        let authorities: Vec<_> = (0..10)
            .map(|id| UintAuthorityId(id).to_public_key())
            .collect();

        let state = PartyState {
            validator_started: vec![SessionId(2)],
            early_started: vec![SessionId(3)],
            non_validator_started: vec![],
            stopped: vec![],
        };

        test.set_now(
            Some((SessionId(0), authorities.clone())),
            Some(Some(UintAuthorityId(0).to_public_key())),
        )
        .await
        .set_now(Some((SessionId(1), authorities.clone())), None)
        .await
        .set_now(Some((SessionId(2), authorities.clone())), None)
        .await
        .set_now(Some((SessionId(3), authorities)), None)
        .await
        .set_best_and_finalized_block(SESSION_PERIOD * 2, SESSION_PERIOD * 2)
        .await
        .run_party(party)
        .expect_session_states_at_block(61, state)
        .run_for_n_blocks(1)
        .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn party_dont_start_session_for_node_non_in_authorities() {
        let (test, party) = PartyTest::new(SessionPeriod(SESSION_PERIOD));

        let authorities: Vec<_> = (0..10)
            .map(|id| UintAuthorityId(id).to_public_key())
            .collect();

        let state_1 = PartyState {
            validator_started: vec![SessionId(0)],
            early_started: vec![],
            non_validator_started: vec![],
            stopped: vec![],
        };

        let state_2 = PartyState {
            validator_started: vec![SessionId(0)],
            early_started: vec![],
            non_validator_started: vec![SessionId(1)],
            stopped: vec![SessionId(0)],
        };

        test.set_authorities_for_session_at_block(0, authorities.clone(), SessionId(0))
            .set_authorities_for_session_at_block(25, authorities[1..].to_vec(), SessionId(1))
            .set_node_id_for_session_at_block(0, Some(UintAuthorityId(0).to_public_key()))
            .expect_session_states_at_block(24, state_1)
            .expect_session_states_at_block(29, state_2)
            .run_party(party)
            .run_for_n_blocks(SESSION_PERIOD)
            .await;
    }
}
