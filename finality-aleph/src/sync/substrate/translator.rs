use std::fmt::{Display, Error as FmtError, Formatter};

use aleph_primitives::BlockNumber;
use sp_runtime::traits::{Block, Header};

use crate::{
    justification::AlephJustification,
    sync::{
        substrate::{
            chain_status::{Error as ChainStatusError, SubstrateChainStatus},
            BlockId, Justification, JustificationTranslator,
        },
        BlockStatus, ChainStatus,
    },
};

pub enum Error<B: Block> {
    ChainStatus(ChainStatusError<B>),
    NoBlock,
    AlreadyJustified,
}

impl<B: Block> Display for Error<B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use Error::*;
        match self {
            ChainStatus(e) => {
                write!(f, "error retrieving block status: {}", e)
            }
            NoBlock => write!(f, "block not present"),
            AlreadyJustified => write!(f, "block already justified"),
        }
    }
}

impl<B: Block> From<ChainStatusError<B>> for Error<B> {
    fn from(value: ChainStatusError<B>) -> Self {
        Error::ChainStatus(value)
    }
}

impl<B> JustificationTranslator<B::Header> for SubstrateChainStatus<B>
where
    B: Block,
    B::Header: Header<Number = BlockNumber>,
{
    type Error = Error<B>;

    fn translate(
        &self,
        aleph_justification: AlephJustification,
        hash: <B::Header as Header>::Hash,
        number: BlockNumber,
    ) -> Result<Justification<B::Header>, Self::Error> {
        use BlockStatus::*;
        let block_id = BlockId::new(hash, number);
        match self.status_of(block_id)? {
            Justified(_) => Err(Error::AlreadyJustified),
            Unknown => Err(Error::NoBlock),
            Present(header) => Ok(Justification::aleph_justification(
                header,
                aleph_justification,
            )),
        }
    }
}
