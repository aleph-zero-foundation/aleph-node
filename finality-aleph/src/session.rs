use aleph_primitives::BlockNumber;
use codec::{Decode, Encode};
use sp_runtime::traits::Block;

use crate::NumberFor;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct SessionBoundaries<B: Block> {
    first_block: NumberFor<B>,
    last_block: NumberFor<B>,
}

impl<B: Block> SessionBoundaries<B> {
    pub fn new(session_id: SessionId, period: SessionPeriod) -> Self {
        SessionBoundaries {
            first_block: first_block_of_session(session_id, period).into(),
            last_block: last_block_of_session(session_id, period).into(),
        }
    }

    pub fn first_block(&self) -> NumberFor<B> {
        self.first_block
    }

    pub fn last_block(&self) -> NumberFor<B> {
        self.last_block
    }
}

fn first_block_of_session(session_id: SessionId, period: SessionPeriod) -> BlockNumber {
    session_id.0 * period.0
}

fn last_block_of_session(session_id: SessionId, period: SessionPeriod) -> BlockNumber {
    (session_id.0 + 1) * period.0 - 1
}

fn session_id_from_block_num(num: BlockNumber, period: SessionPeriod) -> SessionId {
    SessionId(num / period.0)
}

pub struct SessionBoundaryInfo {
    session_period: SessionPeriod,
}

/// Struct for getting the session boundaries.
impl SessionBoundaryInfo {
    pub fn new(session_period: SessionPeriod) -> Self {
        Self { session_period }
    }

    /// Returns session id of the session that block belongs to.
    pub fn session_id_from_block_num(&self, n: BlockNumber) -> SessionId {
        session_id_from_block_num(n, self.session_period)
    }

    /// Returns block number which is the last block of the session.
    pub fn last_block_of_session(&self, session_id: SessionId) -> BlockNumber {
        last_block_of_session(session_id, self.session_period)
    }

    /// Returns block number which is the first block of the session.
    pub fn first_block_of_session(&self, session_id: SessionId) -> BlockNumber {
        first_block_of_session(session_id, self.session_period)
    }
}

#[cfg(test)]
pub mod testing {
    use aleph_primitives::SessionAuthorityData;
    use sp_runtime::testing::UintAuthorityId;

    pub fn authority_data(from: u32, to: u32) -> SessionAuthorityData {
        SessionAuthorityData::new(
            (from..to)
                .map(|id| UintAuthorityId(id.into()).to_public_key())
                .collect(),
            None,
        )
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode)]
pub struct SessionId(pub u32);

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode)]
pub struct SessionPeriod(pub u32);
