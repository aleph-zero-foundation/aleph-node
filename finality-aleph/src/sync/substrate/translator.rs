use std::fmt::{Display, Error as FmtError, Formatter};

use crate::{
    aleph_primitives::Header as AlephHeader,
    justification::AlephJustification,
    sync::{
        substrate::{
            chain_status::{Error as ChainStatusError, SubstrateChainStatus},
            BlockId, Justification, JustificationTranslator,
        },
        BlockStatus, ChainStatus,
    },
};

#[derive(Debug)]
pub enum Error {
    ChainStatus(ChainStatusError),
    NoBlock,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use Error::*;
        match self {
            ChainStatus(e) => {
                write!(f, "error retrieving block status: {}", e)
            }
            NoBlock => write!(f, "block not present"),
        }
    }
}

impl From<ChainStatusError> for Error {
    fn from(value: ChainStatusError) -> Self {
        Error::ChainStatus(value)
    }
}

impl JustificationTranslator<AlephHeader> for SubstrateChainStatus {
    type Error = Error;

    fn translate(
        &self,
        aleph_justification: AlephJustification,
        block_id: BlockId<AlephHeader>,
    ) -> Result<Justification<AlephHeader>, Self::Error> {
        use BlockStatus::*;
        match self.status_of(block_id)? {
            Justified(Justification { header, .. }) | Present(header) => Ok(
                Justification::aleph_justification(header, aleph_justification),
            ),
            Unknown => Err(Error::NoBlock),
        }
    }
}
