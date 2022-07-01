use codec::{Decode, Encode};
use sp_runtime::{traits::Block, SaturatedConversion};

use crate::NumberFor;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct SessionBoundaries<B: Block> {
    first_block: NumberFor<B>,
    last_block: NumberFor<B>,
}

impl<B: Block> SessionBoundaries<B> {
    pub fn new(session_id: SessionId, period: SessionPeriod) -> Self {
        SessionBoundaries {
            first_block: first_block_of_session::<B>(session_id, period),
            last_block: last_block_of_session::<B>(session_id, period),
        }
    }

    pub fn first_block(&self) -> NumberFor<B> {
        self.first_block
    }

    pub fn last_block(&self) -> NumberFor<B> {
        self.last_block
    }
}

pub fn first_block_of_session<B: Block>(
    session_id: SessionId,
    period: SessionPeriod,
) -> NumberFor<B> {
    (session_id.0 * period.0).into()
}

pub fn last_block_of_session<B: Block>(
    session_id: SessionId,
    period: SessionPeriod,
) -> NumberFor<B> {
    ((session_id.0 + 1) * period.0 - 1).into()
}

pub fn session_id_from_block_num<B: Block>(num: NumberFor<B>, period: SessionPeriod) -> SessionId {
    SessionId(num.saturated_into::<u32>() / period.0)
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode)]
pub struct SessionId(pub u32);

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode)]
pub struct SessionPeriod(pub u32);
