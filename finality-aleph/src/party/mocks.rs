use std::{
    collections::HashSet,
    fmt::{Debug, Display, Error as FmtError, Formatter},
    hash::Hash,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;

use crate::{
    oneshot,
    party::{
        backup::ABFTBackup,
        manager::AuthorityTask,
        traits::{Block, ChainState, NodeSessionManager, SessionInfo, SyncState},
    },
    AuthorityId, NodeIndex, SessionId,
};

type AMutex<T> = Arc<Mutex<T>>;

pub struct SimpleBlock;

impl Block for SimpleBlock {
    type Number = u32;
    type Hash = String;
}

#[derive(Clone, Debug)]
pub struct MockChainState {
    pub best_block: AMutex<u32>,
    pub finalized_block: AMutex<u32>,
}

impl MockChainState {
    pub fn new() -> Self {
        Self {
            best_block: Default::default(),
            finalized_block: Default::default(),
        }
    }

    pub fn set_best_block(&self, best_block: u32) {
        *self.best_block.lock().unwrap() = best_block;
    }

    pub fn set_finalized_block(&self, finalized_block: u32) {
        *self.finalized_block.lock().unwrap() = finalized_block;
    }
}

impl ChainState<SimpleBlock> for Arc<MockChainState> {
    fn best_block_number(&self) -> u32 {
        *self.best_block.lock().unwrap()
    }

    fn finalized_number(&self) -> u32 {
        *self.finalized_block.lock().unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct MockSyncState {
    pub is_syncing: AMutex<bool>,
}

impl MockSyncState {
    pub fn new() -> Self {
        Self {
            is_syncing: Arc::new(Mutex::new(false)),
        }
    }

    pub fn _set_is_syncing(&self, is_syncing: bool) {
        *self.is_syncing.lock().unwrap() = is_syncing;
    }
}

impl SyncState<SimpleBlock> for Arc<MockSyncState> {
    fn is_major_syncing(&self) -> bool {
        *self.is_syncing.lock().unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct MockNodeSessionManager {
    pub nonvalidator_session_started: AMutex<HashSet<SessionId>>,
    pub validator_session_started: AMutex<HashSet<SessionId>>,
    pub session_stopped: AMutex<HashSet<SessionId>>,
    pub session_early_started: AMutex<HashSet<SessionId>>,
    pub node_id: AMutex<Option<AuthorityId>>,
}

impl MockNodeSessionManager {
    pub fn new() -> Self {
        Self {
            nonvalidator_session_started: Default::default(),
            validator_session_started: Default::default(),
            session_stopped: Default::default(),
            session_early_started: Default::default(),
            node_id: Default::default(),
        }
    }

    pub fn set_node_id(&self, node_id: Option<AuthorityId>) {
        *self.node_id.lock().unwrap() = node_id;
    }

    fn insert<T: Eq + Hash>(&self, set: AMutex<HashSet<T>>, el: T) {
        let mut x = set.lock().unwrap();
        x.insert(el);
    }
}

pub struct MockNodeSessionManagerError;

impl Display for MockNodeSessionManagerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(f, "mock node session manager error")
    }
}

#[async_trait]
impl NodeSessionManager for Arc<MockNodeSessionManager> {
    type Error = MockNodeSessionManagerError;

    async fn spawn_authority_task_for_session(
        &self,
        session: SessionId,
        node_id: NodeIndex,
        _backup: ABFTBackup,
        _authorities: &[AuthorityId],
    ) -> AuthorityTask {
        self.insert(self.validator_session_started.clone(), session);

        let (exit, _) = oneshot::channel();
        let handle = async { Ok(()) };

        AuthorityTask::new(Box::pin(handle), node_id, exit)
    }

    async fn early_start_validator_session(
        &self,
        session: SessionId,
        _node_id: NodeIndex,
        _authorities: &[AuthorityId],
    ) -> Result<(), Self::Error> {
        self.insert(self.session_early_started.clone(), session);

        Ok(())
    }

    fn start_nonvalidator_session(
        &self,
        session: SessionId,
        _authorities: &[AuthorityId],
    ) -> Result<(), Self::Error> {
        self.insert(self.nonvalidator_session_started.clone(), session);

        Ok(())
    }

    fn stop_session(&self, session: SessionId) -> Result<(), Self::Error> {
        self.insert(self.session_stopped.clone(), session);

        Ok(())
    }

    async fn node_idx(&self, authorities: &[AuthorityId]) -> Option<NodeIndex> {
        let id = &*self.node_id.lock().unwrap();

        if let Some(id) = id {
            if let Some(idx) = authorities.iter().position(|x| x == id) {
                // doesnt mather for tests what nodeindex we are
                return Some(NodeIndex(idx));
            }

            return None;
        }

        None
    }
}

pub struct MockSessionInfo {
    pub session_period: u32,
}

impl MockSessionInfo {
    pub fn new(session_period: u32) -> Self {
        Self { session_period }
    }
}

impl SessionInfo<SimpleBlock> for MockSessionInfo {
    fn session_id_from_block_num(&self, n: u32) -> SessionId {
        SessionId(n / self.session_period)
    }

    fn last_block_of_session(&self, session_id: SessionId) -> u32 {
        (session_id.0 + 1) * self.session_period - 1
    }

    fn first_block_of_session(&self, session_id: SessionId) -> u32 {
        session_id.0 * self.session_period
    }
}
