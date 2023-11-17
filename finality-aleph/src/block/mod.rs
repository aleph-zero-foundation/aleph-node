use std::fmt::{Debug, Display, Error as FmtError, Formatter};

use parity_scale_codec::{Codec, Decode, Encode};

use crate::{BlockHash, BlockNumber};

#[cfg(test)]
pub mod mock;
pub mod substrate;

/// The identifier of a block, the least amount of knowledge we can have about a block.
#[derive(PartialEq, Eq, Clone, Debug, Encode, Decode, Hash)]
pub struct BlockId {
    hash: BlockHash,
    number: BlockNumber,
}

impl BlockId {
    pub fn new(hash: BlockHash, number: BlockNumber) -> Self {
        BlockId { hash, number }
    }

    pub fn number(&self) -> BlockNumber {
        self.number
    }

    pub fn hash(&self) -> BlockHash {
        self.hash
    }
}

impl From<(BlockHash, BlockNumber)> for BlockId {
    fn from(pair: (BlockHash, BlockNumber)) -> Self {
        BlockId::new(pair.0, pair.1)
    }
}

impl Display for BlockId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(f, "#{} ({})", self.number, self.hash,)
    }
}

/// The unverified header of a block, containing information about the parent relation.
pub trait UnverifiedHeader: Clone + Codec + Debug + Send + Sync + Eq + 'static {
    /// The identifier of this block.
    fn id(&self) -> BlockId;
}

/// The header of a block, containing information about the parent relation.
pub trait Header: Clone + Codec + Debug + Send + Sync + 'static {
    type Unverified: UnverifiedHeader;

    /// The identifier of this block.
    fn id(&self) -> BlockId;

    /// The identifier of this block's parent.
    fn parent_id(&self) -> Option<BlockId>;

    /// Return an unverified version of this, for sending over the network.
    fn into_unverified(self) -> Self::Unverified;
}

pub trait UnverifiedJustification: Clone + Codec + Send + Sync + Debug + 'static {
    type UnverifiedHeader: UnverifiedHeader;

    /// The header of the block.
    fn header(&self) -> &Self::UnverifiedHeader;
}

/// The verified justification of a block, including a header.
pub trait Justification: Clone + Send + Sync + Debug + 'static {
    type Header: Header;
    type Unverified: UnverifiedJustification<
        UnverifiedHeader = <Self::Header as Header>::Unverified,
    >;

    /// The header of the block.
    fn header(&self) -> &Self::Header;

    /// Return an unverified version of this, for sending over the network.
    fn into_unverified(self) -> Self::Unverified;
}

/// A verifier of justifications.
pub trait JustificationVerifier<J: Justification> {
    type Error: Display + Debug;

    /// Verifies the raw justification and returns a full justification if successful, otherwise an
    /// error.
    fn verify_justification(&mut self, justification: J::Unverified) -> Result<J, Self::Error>;
}

pub type UnverifiedHeaderFor<J> = <<J as Justification>::Header as Header>::Unverified;

pub trait EquivocationProof: Display {
    /// Returns if we are the offender.
    fn are_we_equivocating(&self) -> bool;
}

pub struct VerifiedHeader<H: Header, P: EquivocationProof> {
    pub header: H,
    pub maybe_equivocation_proof: Option<P>,
}

/// A verifier of headers.
pub trait HeaderVerifier<H: Header>: Clone + Send + Sync + 'static {
    type EquivocationProof: EquivocationProof;
    type Error: Display + Debug;

    /// Verifies the raw header and returns a struct containing a full header and possibly
    /// an equivocation proof if successful, otherwise an error.
    /// In case the header comes from a block that we've just authored,
    /// the `just_created` flag must be set to `true`.
    fn verify_header(
        &mut self,
        header: H::Unverified,
        just_created: bool,
    ) -> Result<VerifiedHeader<H, Self::EquivocationProof>, Self::Error>;
}

/// The block, including a header.
pub trait Block: Clone + Codec + Debug + Send + Sync + 'static {
    type UnverifiedHeader: UnverifiedHeader;

    /// The header of the block.
    fn header(&self) -> &Self::UnverifiedHeader;
}

/// The block importer.
pub trait BlockImport<B>: Send + 'static {
    /// Import the block.
    fn import_block(&mut self, block: B);
}

/// A facility for finalizing blocks using justifications.
pub trait Finalizer<J: Justification> {
    type Error: Display;

    /// Finalize a block using this justification. Since the justification contains the header, we
    /// don't need to additionally specify the block.
    fn finalize(&self, justification: J) -> Result<(), Self::Error>;
}

/// A notification about the chain status changing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChainStatusNotification<H: Header> {
    /// A block has been imported.
    BlockImported(H),
    /// A block has been finalized.
    BlockFinalized(H),
}

/// A stream of notifications about the chain status in the database changing.
/// We assume that this will return all the events, otherwise we will end up with a broken state.
#[async_trait::async_trait]
pub trait ChainStatusNotifier<H: Header> {
    type Error: Debug + Display;

    /// Returns a chain status notification when it is available.
    /// This method's implementation must be cancellation safe.
    async fn next(&mut self) -> Result<ChainStatusNotification<H>, Self::Error>;
}

/// The status of a block in the database.
pub enum BlockStatus<J: Justification> {
    /// The block is justified and thus finalized.
    Justified(J),
    /// The block is present, might be finalized if a descendant is justified.
    Present(J::Header),
    /// The block is not known.
    Unknown,
}

/// FinalizationStatus of the block
pub enum FinalizationStatus<J: Justification> {
    /// The block is finalized by justification.
    FinalizedWithJustification(J),
    /// The block is finalized because one of its children is finalized.
    FinalizedByDescendant(J::Header),
    /// The block is not finalized
    NotFinalized,
}

impl<J: Justification> FinalizationStatus<J> {
    pub fn has_justification(&self) -> Option<J> {
        use FinalizationStatus::*;
        match self {
            FinalizedWithJustification(just) => Some(just.clone()),
            _ => None,
        }
    }
}

/// The knowledge about the chain status.
pub trait ChainStatus<B, J>: Clone + Send + Sync + 'static
where
    J: Justification,
    B: Block<UnverifiedHeader = UnverifiedHeaderFor<J>>,
{
    type Error: Display;

    /// The status of the block.
    fn status_of(&self, id: BlockId) -> Result<BlockStatus<J>, Self::Error>;

    /// Export a copy of the block.
    fn block(&self, id: BlockId) -> Result<Option<B>, Self::Error>;

    /// The justification at this block number, if we have it otherwise just block id if
    /// the block is finalized without justification. Should return NotFinalized variant if
    /// the request is above the top finalized.
    fn finalized_at(&self, number: u32) -> Result<FinalizationStatus<J>, Self::Error>;

    /// The header of the best block.
    fn best_block(&self) -> Result<J::Header, Self::Error>;

    /// The justification of the top finalized block.
    fn top_finalized(&self) -> Result<J, Self::Error>;

    /// Children of the specified block.
    fn children(&self, id: BlockId) -> Result<Vec<J::Header>, Self::Error>;
}
