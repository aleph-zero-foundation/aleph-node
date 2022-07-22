use std::{marker::PhantomData, sync::Arc};

use sc_client_api::Backend;
use sp_runtime::traits::{Block as BlockT, NumberFor, SaturatedConversion};

use crate::{
    party::traits::{Block, ChainState, SessionInfo},
    ClientForAleph, SessionId, SessionPeriod,
};

pub struct ChainStateImpl<B, BE, CFA>
where
    B: BlockT,
    BE: Backend<B>,
    CFA: ClientForAleph<B, BE>,
{
    pub client: Arc<CFA>,
    pub _phantom: PhantomData<(B, BE)>,
}

impl<B, BE, CFA> ChainState<B> for ChainStateImpl<B, BE, CFA>
where
    B: BlockT,
    BE: Backend<B>,
    CFA: ClientForAleph<B, BE>,
{
    fn best_block_number(&self) -> <B as Block>::Number {
        self.client.info().best_number
    }
    fn finalized_number(&self) -> <B as Block>::Number {
        self.client.info().finalized_number
    }
}

pub struct SessionInfoImpl {
    session_period: SessionPeriod,
}

impl SessionInfoImpl {
    pub fn new(session_period: SessionPeriod) -> Self {
        Self { session_period }
    }
}

impl<B: BlockT> SessionInfo<B> for SessionInfoImpl {
    fn session_id_from_block_num(&self, n: NumberFor<B>) -> SessionId {
        SessionId(n.saturated_into::<u32>() / self.session_period.0)
    }

    fn last_block_of_session(&self, session_id: SessionId) -> NumberFor<B> {
        ((session_id.0 + 1) * self.session_period.0 - 1).into()
    }

    fn first_block_of_session(&self, session_id: SessionId) -> NumberFor<B> {
        (session_id.0 * self.session_period.0).into()
    }
}
