use std::fmt::{Debug, Display};

use parity_scale_codec::{Decode, Encode};
use sc_consensus::import_queue::{ImportQueueService, IncomingBlock};
use sp_consensus::BlockOrigin;
use sp_runtime::traits::{CheckedSub, Header as SubstrateHeader, One};

use crate::{
    aleph_primitives::{Block, BlockNumber, Header},
    sync::{Block as BlockT, BlockImport, Header as HeaderT, Justification as JustificationT},
    AlephJustification, BlockId,
};

mod chain_status;
mod finalizer;
mod status_notifier;
mod translator;
mod verification;

pub use chain_status::SubstrateChainStatus;
pub use status_notifier::SubstrateChainStatusNotifier;
pub use translator::Error as TranslateError;
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

impl<H: SubstrateHeader<Number = BlockNumber>> HeaderT for H {
    type Identifier = BlockId<H>;

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

/// Proper `AlephJustification` or a variant indicating virtual justification
/// for the genesis block, which is the only block that can be the top finalized
/// block with no proper justification.
#[derive(Clone, Debug, Encode, Decode)]
pub enum InnerJustification {
    AlephJustification(AlephJustification),
    Genesis,
}

/// A justification, including the related header.
#[derive(Clone, Debug, Encode, Decode)]
pub struct Justification<H: SubstrateHeader<Number = BlockNumber>> {
    header: H,
    inner_justification: InnerJustification,
}

impl<H: SubstrateHeader<Number = BlockNumber>> Justification<H> {
    pub fn aleph_justification(header: H, aleph_justification: AlephJustification) -> Self {
        Justification {
            header,
            inner_justification: InnerJustification::AlephJustification(aleph_justification),
        }
    }

    pub fn genesis_justification(header: H) -> Self {
        Justification {
            header,
            inner_justification: InnerJustification::Genesis,
        }
    }
}

impl<H: SubstrateHeader<Number = BlockNumber>> HeaderT for Justification<H> {
    type Identifier = BlockId<H>;

    fn id(&self) -> Self::Identifier {
        self.header().id()
    }

    fn parent_id(&self) -> Option<Self::Identifier> {
        self.header().parent_id()
    }
}

impl<H: SubstrateHeader<Number = BlockNumber>> JustificationT for Justification<H> {
    type Header = H;
    type Unverified = Self;

    fn header(&self) -> &Self::Header {
        &self.header
    }

    fn into_unverified(self) -> Self::Unverified {
        self
    }
}

/// Translates raw aleph justifications into ones acceptable to sync.
pub trait JustificationTranslator<H: SubstrateHeader<Number = BlockNumber>>: Send + Sync {
    type Error: Display + Debug;

    fn translate(
        &self,
        raw_justification: AlephJustification,
        block_id: BlockId<H>,
    ) -> Result<Justification<H>, Self::Error>;
}
