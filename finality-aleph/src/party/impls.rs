use std::{marker::PhantomData, sync::Arc};

use aleph_primitives::BlockNumber;
use sc_client_api::Backend;
use sc_network::NetworkService;
use sc_network_common::ExHashT;
use sp_consensus::SyncOracle;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};

use crate::{
    party::traits::{ChainState, SessionInfo, SyncState},
    session::{first_block_of_session, last_block_of_session, session_id_from_block_num},
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

impl<B, BE, CFA> ChainState for ChainStateImpl<B, BE, CFA>
where
    B: BlockT,
    B::Header: HeaderT<Number = BlockNumber>,
    BE: Backend<B>,
    CFA: ClientForAleph<B, BE>,
{
    fn best_block_number(&self) -> BlockNumber {
        self.client.info().best_number
    }
    fn finalized_number(&self) -> BlockNumber {
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

impl SessionInfo for SessionInfoImpl {
    fn session_id_from_block_num(&self, n: BlockNumber) -> SessionId {
        session_id_from_block_num(n, self.session_period)
    }

    fn last_block_of_session(&self, session_id: SessionId) -> BlockNumber {
        last_block_of_session(session_id, self.session_period)
    }

    fn first_block_of_session(&self, session_id: SessionId) -> BlockNumber {
        first_block_of_session(session_id, self.session_period)
    }
}

impl<B: BlockT, H: ExHashT> SyncState for Arc<NetworkService<B, H>> {
    fn is_major_syncing(&self) -> bool {
        NetworkService::is_major_syncing(self)
    }
}
