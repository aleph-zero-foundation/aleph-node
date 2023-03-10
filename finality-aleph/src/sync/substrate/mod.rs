use std::hash::{Hash, Hasher};

use aleph_primitives::BlockNumber;
use codec::{Decode, Encode};
use sp_runtime::traits::{CheckedSub, Header as SubstrateHeader, One};

use crate::{
    sync::{BlockIdentifier, Header, Justification as JustificationT},
    AlephJustification,
};

mod chain_status;
mod finalizer;
mod status_notifier;
mod verification;

pub use verification::SessionVerifier;

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub struct BlockId<H: SubstrateHeader<Number = BlockNumber>> {
    hash: H::Hash,
    number: H::Number,
}

/// An identifier uniquely specifying a block and its height.
impl<SH: SubstrateHeader<Number = BlockNumber>> Hash for BlockId<SH> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.hash.hash(state);
        self.number.hash(state);
    }
}

impl<H: SubstrateHeader<Number = BlockNumber>> BlockIdentifier for BlockId<H> {
    fn number(&self) -> u32 {
        self.number
    }
}

impl<H: SubstrateHeader<Number = BlockNumber>> Header for H {
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

/// A justification, including the related header.
#[derive(Clone, Debug, Encode, Decode)]
pub struct Justification<H: SubstrateHeader<Number = BlockNumber>> {
    header: H,
    raw_justification: AlephJustification,
}

impl<H: SubstrateHeader<Number = BlockNumber>> Header for Justification<H> {
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
