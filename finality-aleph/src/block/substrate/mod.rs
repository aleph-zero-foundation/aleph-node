use std::time::Instant;

use sc_consensus::import_queue::{ImportQueueService, IncomingBlock};
use sp_consensus::BlockOrigin;
use sp_runtime::traits::{CheckedSub, Header as _, One};

use crate::{
    aleph_primitives::{Block, Header},
    block::{Block as BlockT, BlockId, BlockImport, Header as HeaderT, UnverifiedHeader},
    metrics::Checkpoint,
    TimingBlockMetrics,
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

const LOG_TARGET: &str = "aleph-substrate";

impl UnverifiedHeader for Header {
    fn id(&self) -> BlockId {
        BlockId {
            hash: self.hash(),
            number: *self.number(),
        }
    }
}

impl HeaderT for Header {
    type Unverified = Self;

    fn id(&self) -> BlockId {
        BlockId {
            hash: self.hash(),
            number: *self.number(),
        }
    }

    fn parent_id(&self) -> Option<BlockId> {
        let number = self.number().checked_sub(&One::one())?;
        Some(BlockId {
            hash: *self.parent_hash(),
            number,
        })
    }

    fn into_unverified(self) -> Self::Unverified {
        self
    }
}

/// Wrapper around the trait object that we get from Substrate.
pub struct BlockImporter {
    importer: Box<dyn ImportQueueService<Block>>,
    metrics: TimingBlockMetrics,
}

impl BlockImporter {
    pub fn new(importer: Box<dyn ImportQueueService<Block>>) -> Self {
        Self {
            importer,
            metrics: TimingBlockMetrics::Noop,
        }
    }
    pub fn attach_metrics(&mut self, metrics: TimingBlockMetrics) {
        self.metrics = metrics;
    }
}

impl BlockImport<Block> for BlockImporter {
    fn import_block(&mut self, block: Block) {
        let origin = BlockOrigin::NetworkBroadcast;
        let hash = block.header.hash();
        let incoming_block = IncomingBlock::<Block> {
            hash,
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
        self.metrics
            .report_block_if_not_present(hash, Instant::now(), Checkpoint::Importing);
        self.importer.import_blocks(origin, vec![incoming_block]);
    }
}

impl BlockT for Block {
    type UnverifiedHeader = Header;

    /// The header of the block.
    fn header(&self) -> &Self::UnverifiedHeader {
        &self.header
    }
}
