use std::{marker::PhantomData, sync::Arc};

use sc_client_api::Backend;
use sc_network_sync::SyncingService;
use sp_consensus::SyncOracle;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};

use crate::{
    aleph_primitives::BlockNumber,
    party::traits::{ChainState, SyncState},
    ClientForAleph,
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

impl<B: BlockT> SyncState for Arc<SyncingService<B>> {
    fn is_major_syncing(&self) -> bool {
        SyncingService::is_major_syncing(self)
    }
}
