use std::fmt::Display;

use async_trait::async_trait;

use crate::{
    aleph_primitives::BlockNumber,
    party::{backup::ABFTBackup, manager::AuthorityTask},
    AuthorityId, NodeIndex, SessionId,
};

/// Abstraction of the chain state.
pub trait ChainState {
    /// Returns best block number.
    fn best_block_number(&self) -> BlockNumber;
    /// Returns last finalized block number.
    fn finalized_number(&self) -> BlockNumber;
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
    fn early_start_validator_session(
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
    fn node_idx(&self, authorities: &[AuthorityId]) -> Option<NodeIndex>;
}
