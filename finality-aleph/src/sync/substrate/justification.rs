use std::fmt::{Debug, Display, Error as FmtError, Formatter};

use parity_scale_codec::{Decode, Encode};

use crate::{
    aleph_primitives::Header,
    justification::AlephJustification,
    sync::{
        substrate::{
            chain_status::{Error as ChainStatusError, SubstrateChainStatus},
            BlockId,
        },
        BlockStatus, ChainStatus, Justification as JustificationT, UnverifiedJustification,
    },
};

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
pub struct Justification {
    pub header: Header,
    pub inner_justification: InnerJustification,
}

impl Justification {
    pub fn aleph_justification(header: Header, aleph_justification: AlephJustification) -> Self {
        Justification {
            header,
            inner_justification: InnerJustification::AlephJustification(aleph_justification),
        }
    }

    pub fn genesis_justification(header: Header) -> Self {
        Justification {
            header,
            inner_justification: InnerJustification::Genesis,
        }
    }

    pub fn into_inner(self) -> InnerJustification {
        self.inner_justification
    }
}

impl UnverifiedJustification for Justification {
    type Header = Header;

    fn header(&self) -> &Self::Header {
        &self.header
    }
}

impl JustificationT for Justification {
    type Header = Header;
    type Unverified = Self;

    fn header(&self) -> &Self::Header {
        &self.header
    }

    fn into_unverified(self) -> Self::Unverified {
        self
    }
}

#[derive(Debug)]
pub enum TranslateError {
    ChainStatus(ChainStatusError),
    NoBlock,
}

impl Display for TranslateError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use TranslateError::*;
        match self {
            ChainStatus(e) => {
                write!(f, "error retrieving block status: {e}")
            }
            NoBlock => write!(f, "block not present"),
        }
    }
}

impl From<ChainStatusError> for TranslateError {
    fn from(value: ChainStatusError) -> Self {
        TranslateError::ChainStatus(value)
    }
}

/// Translates raw aleph justifications into ones acceptable to sync.
#[derive(Clone)]
pub struct JustificationTranslator {
    chain_status: SubstrateChainStatus,
}

impl JustificationTranslator {
    pub fn new(chain_status: SubstrateChainStatus) -> Self {
        Self { chain_status }
    }

    pub fn translate(
        &self,
        aleph_justification: AlephJustification,
        block_id: BlockId,
    ) -> Result<Justification, TranslateError> {
        use BlockStatus::*;
        match self.chain_status.status_of(block_id)? {
            Justified(Justification { header, .. }) | Present(header) => Ok(
                Justification::aleph_justification(header, aleph_justification),
            ),
            Unknown => Err(TranslateError::NoBlock),
        }
    }
}
