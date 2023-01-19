use codec::{Decode, Encode};
use sp_runtime::{
    traits::{AtLeast32BitUnsigned, Block},
    SaturatedConversion,
};

use crate::NumberFor;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct SessionBoundaries<B: Block> {
    first_block: NumberFor<B>,
    last_block: NumberFor<B>,
}

impl<B: Block> SessionBoundaries<B> {
    pub fn new(session_id: SessionId, period: SessionPeriod) -> Self {
        SessionBoundaries {
            first_block: first_block_of_session(session_id, period),
            last_block: last_block_of_session(session_id, period),
        }
    }

    pub fn first_block(&self) -> NumberFor<B> {
        self.first_block
    }

    pub fn last_block(&self) -> NumberFor<B> {
        self.last_block
    }
}

pub fn first_block_of_session<N: AtLeast32BitUnsigned>(
    session_id: SessionId,
    period: SessionPeriod,
) -> N {
    (session_id.0 * period.0).into()
}

pub fn last_block_of_session<N: AtLeast32BitUnsigned>(
    session_id: SessionId,
    period: SessionPeriod,
) -> N {
    ((session_id.0 + 1) * period.0 - 1).into()
}

pub fn session_id_from_block_num<N: AtLeast32BitUnsigned>(
    num: N,
    period: SessionPeriod,
) -> SessionId {
    SessionId((num / period.0.into()).saturated_into())
}

#[cfg(test)]
pub mod testing {
    use aleph_primitives::SessionAuthorityData;
    use sp_runtime::testing::UintAuthorityId;

    pub fn authority_data(from: u64, to: u64) -> SessionAuthorityData {
        SessionAuthorityData::new(
            (from..to)
                .map(|id| UintAuthorityId(id).to_public_key())
                .collect(),
            None,
        )
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode)]
pub struct SessionId(pub u32);

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode)]
pub struct SessionPeriod(pub u32);
