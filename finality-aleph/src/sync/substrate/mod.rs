use std::fmt::Debug;

use parity_scale_codec::{Decode, Encode};
use sc_consensus::import_queue::{ImportQueueService, IncomingBlock};
use sp_consensus::BlockOrigin;
use sp_runtime::traits::{CheckedSub, Header as _, One};

use crate::{
    aleph_primitives::{Block, Header},
    sync::{Block as BlockT, BlockImport, Header as HeaderT},
    BlockId,
};

mod chain_status;
mod finalizer;
mod justification;
mod status_notifier;
mod verification;

pub use chain_status::SubstrateChainStatus;
pub use justification::{
    InnerJustification, Justification, JustificationTranslator, TranslateError,
};
pub use status_notifier::SubstrateChainStatusNotifier;
pub use verification::{SessionVerifier, SubstrateFinalizationInfo, VerifierCache};

/// Contains the actual Substrate Block and all additional data required for Substrate sync.
#[derive(Clone, Debug, Encode, Decode)]
pub struct SubstrateSyncBlock {
    inner: Block,
    indexed_body: Option<Vec<Vec<u8>>>,
}

/// Wrapper around the trait object that we get from Substrate.
pub struct BlockImporter(pub Box<dyn ImportQueueService<Block>>);

impl BlockImport<SubstrateSyncBlock> for BlockImporter {
    fn import_block(&mut self, block: SubstrateSyncBlock) {
        let origin = BlockOrigin::NetworkBroadcast;
        let incoming_block = IncomingBlock::<Block> {
            hash: block.inner.header.hash(),
            header: Some(block.inner.header),
            body: Some(block.inner.extrinsics),
            indexed_body: block.indexed_body,
            justifications: None,
            origin: None,
            allow_missing_state: false,
            skip_execution: false,
            import_existing: false,
            state: None,
        };
        self.0.import_blocks(origin, vec![incoming_block]);
    }
}

impl HeaderT for Header {
    type Identifier = BlockId<Header>;

    fn id(&self) -> Self::Identifier {
        BlockId {
            hash: self.hash(),
            number: *self.number(),
        }
    }

    fn parent_id(&self) -> Option<Self::Identifier> {
        let number = self.number().checked_sub(&One::one())?;
        Some(BlockId {
            hash: *self.parent_hash(),
            number,
        })
    }
}

impl BlockT for SubstrateSyncBlock {
    type Header = Header;

    /// The header of the block.
    fn header(&self) -> &Self::Header {
        &self.inner.header
    }
}
