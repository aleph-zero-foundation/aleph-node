use aleph_primitives::BlockNumber;
use codec::{Decode, Encode};

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct SessionBoundaries {
    first_block: BlockNumber,
    last_block: BlockNumber,
}

impl SessionBoundaries {
    pub fn first_block(&self) -> BlockNumber {
        self.first_block
    }

    pub fn last_block(&self) -> BlockNumber {
        self.last_block
    }
}

pub struct SessionBoundaryInfo {
    session_period: SessionPeriod,
}

/// Struct for getting the session boundaries.
impl SessionBoundaryInfo {
    pub fn new(session_period: SessionPeriod) -> Self {
        Self { session_period }
    }

    pub fn boundaries_for_session(&self, session_id: SessionId) -> SessionBoundaries {
        SessionBoundaries {
            first_block: self.first_block_of_session(session_id),
            last_block: self.last_block_of_session(session_id),
        }
    }

    /// Returns session id of the session that block belongs to.
    pub fn session_id_from_block_num(&self, n: BlockNumber) -> SessionId {
        SessionId(n / self.session_period.0)
    }

    /// Returns block number which is the last block of the session.
    pub fn last_block_of_session(&self, session_id: SessionId) -> BlockNumber {
        (session_id.0 + 1) * self.session_period.0 - 1
    }

    /// Returns block number which is the first block of the session.
    pub fn first_block_of_session(&self, session_id: SessionId) -> BlockNumber {
        session_id.0 * self.session_period.0
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

impl SessionId {
    /// The id of the session following this one.
    pub fn next(&self) -> Self {
        SessionId(self.0 + 1)
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode)]
pub struct SessionPeriod(pub u32);
