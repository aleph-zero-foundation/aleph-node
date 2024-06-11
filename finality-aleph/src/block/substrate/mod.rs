use sc_consensus::import_queue::{ImportQueueService, IncomingBlock};
use sp_consensus::BlockOrigin;
use sp_runtime::traits::{CheckedSub, Header as _, One};

use crate::{
    aleph_primitives::{Block, Header},
    block::{Block as BlockT, BlockId, BlockImport, Header as HeaderT, UnverifiedHeader},
    metrics::TimingBlockMetrics,
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
pub use verification::{SubstrateFinalizationInfo, VerifierCache};

use crate::{
    block::{BestBlockSelector, BlockchainEvents},
    metrics::Checkpoint,
};

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
            metrics: TimingBlockMetrics::noop(),
        }
    }

    pub fn attach_metrics(&mut self, metrics: TimingBlockMetrics) {
        self.metrics = metrics;
    }
}

impl BlockImport<Block> for BlockImporter {
    fn import_block(&mut self, block: Block, own: bool) {
        // We only need to distinguish between blocks produced by us and blocks incoming from the network
        // for the purpose of running `FinalityRateMetrics`. We use `BlockOrigin` to make this distinction.
        let origin = match own {
            true => BlockOrigin::Own,
            false => BlockOrigin::NetworkBroadcast,
        };
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
        self.metrics.report_block(hash, Checkpoint::Importing);
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

impl<C: sc_client_api::BlockchainEvents<Block> + Send> BlockchainEvents<Header> for C {
    type ChainStatusNotifier = SubstrateChainStatusNotifier;

    fn chain_status_notifier(&self) -> SubstrateChainStatusNotifier {
        SubstrateChainStatusNotifier::new(
            self.finality_notification_stream(),
            self.every_import_notification_stream(),
        )
    }
}

#[async_trait::async_trait]
impl<SC: sp_consensus::SelectChain<Block>> BestBlockSelector<Header> for SC {
    type Error = sp_consensus::Error;
    async fn select_best(&self) -> Result<Header, Self::Error> {
        self.best_chain().await
    }
}
