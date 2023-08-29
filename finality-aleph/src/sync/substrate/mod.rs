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

/// Wrapper around the trait object that we get from Substrate.
pub struct BlockImporter(pub Box<dyn ImportQueueService<Block>>);

impl BlockImport<Block> for BlockImporter {
    fn import_block(&mut self, block: Block) {
        let origin = BlockOrigin::NetworkBroadcast;
        let incoming_block = IncomingBlock::<Block> {
            hash: block.header.hash(),
            header: Some(block.header),
            body: Some(block.extrinsics),
            indexed_body: None,
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
    type Identifier = BlockId;

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

impl BlockT for Block {
    type Header = Header;

    /// The header of the block.
    fn header(&self) -> &Self::Header {
        &self.header
    }
}
