use std::fmt::{Debug, Display};

use async_trait::async_trait;
use sp_runtime::traits::{Block as BlockT, NumberFor};

use crate::{
    network,
    party::{backup::ABFTBackup, manager::AuthorityTask},
    AuthorityId, NodeIndex, SessionId,
};

/// Minimal abstraction of the block.
pub trait Block {
    type Number: Debug + PartialOrd + Copy;
    type Hash: Debug;
}

impl<T> Block for T
where
    T: BlockT,
{
    type Number = NumberFor<T>;
    type Hash = <T as BlockT>::Hash;
}

/// Abstraction of the chain state.
pub trait ChainState<B: Block> {
    /// Returns best block number.
    fn best_block_number(&self) -> <B as Block>::Number;
    /// Returns last finalized block number.
    fn finalized_number(&self) -> <B as Block>::Number;
}

#[async_trait]
/// Abstraction over session related tasks.
pub trait NodeSessionManager {
    type Error: Display;

    /// Spawns every task needed for an authority to run in a session.
    async fn spawn_authority_task_for_session(
        &self,
        session: SessionId,
        node_id: NodeIndex,
        backup: ABFTBackup,
        authorities: &[AuthorityId],
    ) -> AuthorityTask;

    /// Prepare validator session.
    async fn early_start_validator_session(
        &self,
        session: SessionId,
        node_id: NodeIndex,
        authorities: &[AuthorityId],
    ) -> Result<(), Self::Error>;

    /// Starts nonvalidator session.
    fn start_nonvalidator_session(
        &self,
        session: SessionId,
        authorities: &[AuthorityId],
    ) -> Result<(), Self::Error>;

    /// Terminates the session.
    fn stop_session(&self, session: SessionId) -> Result<(), Self::Error>;

    /// Returns idx of the node if it is in the authority set, None otherwise
    async fn node_idx(&self, authorities: &[AuthorityId]) -> Option<NodeIndex>;
}

pub trait SyncState<B: Block> {
    /// Are we in the process of downloading the chain?
    ///
    /// Like [`RequestBlocks::is_major_syncing`][1].
    ///
    /// [1]: finality_aleph::network::RequestBlocks::is_major_syncing
    fn is_major_syncing(&self) -> bool;
}

impl<B: BlockT, RB> SyncState<B> for RB
where
    RB: network::RequestBlocks<B>,
{
    fn is_major_syncing(&self) -> bool {
        self.is_major_syncing()
    }
}

/// Abstraction of the session boundaries.
pub trait SessionInfo<B: Block> {
    /// Returns session id of the session that block belongs to.
    fn session_id_from_block_num(&self, n: B::Number) -> SessionId;
    /// Returns block number which is the last block of the session.
    fn last_block_of_session(&self, session_id: SessionId) -> B::Number;
    /// Returns block number which is the first block of the session.
    fn first_block_of_session(&self, session_id: SessionId) -> B::Number;
}
