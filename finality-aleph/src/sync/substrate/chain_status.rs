use std::{
    fmt::{Display, Error as FmtError, Formatter},
    sync::Arc,
};

use aleph_primitives::{BlockNumber, ALEPH_ENGINE_ID};
use log::warn;
use sc_client_api::{Backend as _, HeaderBackend};
use sc_service::TFullBackend;
use sp_blockchain::{Backend as _, Error as BackendError, Info};
use sp_runtime::traits::{Block as BlockT, Header as SubstrateHeader};

use crate::{
    justification::backwards_compatible_decode,
    sync::{
        substrate::{BlockId, Justification},
        BlockStatus, ChainStatus, Header, LOG_TARGET,
    },
};

/// What can go wrong when checking chain status
#[derive(Debug)]
pub enum Error<B: BlockT> {
    MissingHash(B::Hash),
    MissingJustification(B::Hash),
    Backend(BackendError),
    MismatchedId,
    NoGenesisBlock,
}

impl<B: BlockT> Display for Error<B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use Error::*;
        match self {
            MissingHash(hash) => {
                write!(
                    f,
                    "data availability problem: no block for existing hash {:?}",
                    hash
                )
            }
            MissingJustification(hash) => {
                write!(
                    f,
                    "data availability problem: no justification for finalized block with hash {:?}",
                    hash
                )
            }
            Backend(e) => {
                write!(f, "substrate backend error {}", e)
            }
            MismatchedId => write!(f, "the block number did not match the block hash"),
            NoGenesisBlock => write!(f, "genesis block not present in DB"),
        }
    }
}

impl<B: BlockT> From<BackendError> for Error<B> {
    fn from(value: BackendError) -> Self {
        Error::Backend(value)
    }
}

/// Substrate implementation of ChainStatus trait
#[derive(Clone)]
pub struct SubstrateChainStatus<B>
where
    B: BlockT,
    B::Header: SubstrateHeader<Number = BlockNumber>,
{
    backend: Arc<TFullBackend<B>>,
    genesis_header: B::Header,
}

impl<B> SubstrateChainStatus<B>
where
    B: BlockT,
    B::Header: SubstrateHeader<Number = BlockNumber>,
{
    pub fn new(backend: Arc<TFullBackend<B>>) -> Result<Self, Error<B>> {
        let hash = backend.blockchain().hash(0)?.ok_or(Error::NoGenesisBlock)?;
        let genesis_header = backend
            .blockchain()
            .header(hash)?
            .ok_or(Error::MissingHash(hash))?;
        Ok(Self {
            backend,
            genesis_header,
        })
    }

    fn info(&self) -> Info<B> {
        self.backend.blockchain().info()
    }

    pub fn hash_for_number(&self, number: BlockNumber) -> Result<Option<B::Hash>, BackendError> {
        self.backend.blockchain().hash(number)
    }

    pub fn header_for_hash(&self, hash: B::Hash) -> Result<Option<B::Header>, BackendError> {
        self.backend.blockchain().header(hash)
    }

    fn header(
        &self,
        id: &<B::Header as Header>::Identifier,
    ) -> Result<Option<B::Header>, Error<B>> {
        let maybe_header = self.header_for_hash(id.hash)?;
        match maybe_header
            .as_ref()
            .map(|header| header.number() == &id.number)
        {
            Some(false) => Err(Error::MismatchedId),
            _ => Ok(maybe_header),
        }
    }

    fn justification(
        &self,
        header: B::Header,
    ) -> Result<Option<Justification<B::Header>>, BackendError> {
        if header == self.genesis_header {
            return Ok(Some(Justification::genesis_justification(header)));
        };
        let encoded_justification = match self
            .backend
            .blockchain()
            .justifications(header.hash())?
            .and_then(|j| j.into_justification(ALEPH_ENGINE_ID))
        {
            Some(justification) => justification,
            None => return Ok(None),
        };

        match backwards_compatible_decode(encoded_justification) {
            Ok(aleph_justification) => Ok(Some(Justification::aleph_justification(
                header,
                aleph_justification,
            ))),
            // This should not happen, as we only import correctly encoded justification.
            Err(e) => {
                warn!(
                    target: LOG_TARGET,
                    "Could not decode stored justification for block {:?}: {}",
                    header.hash(),
                    e
                );
                Ok(None)
            }
        }
    }

    fn best_hash(&self) -> B::Hash {
        self.info().best_hash
    }

    fn finalized_hash(&self) -> B::Hash {
        self.info().finalized_hash
    }
}

impl<B> ChainStatus<Justification<B::Header>> for SubstrateChainStatus<B>
where
    B: BlockT,
    B::Header: SubstrateHeader<Number = BlockNumber>,
{
    type Error = Error<B>;

    fn finalized_at(
        &self,
        number: BlockNumber,
    ) -> Result<Option<Justification<B::Header>>, Self::Error> {
        let id = match self.hash_for_number(number)? {
            Some(hash) => BlockId { hash, number },
            None => return Ok(None),
        };
        match self.status_of(id)? {
            BlockStatus::Justified(justification) => Ok(Some(justification)),
            _ => Ok(None),
        }
    }

    fn status_of(
        &self,
        id: <B::Header as Header>::Identifier,
    ) -> Result<BlockStatus<Justification<B::Header>>, Self::Error> {
        let header = match self.header(&id)? {
            Some(header) => header,
            None => return Ok(BlockStatus::Unknown),
        };

        if let Some(justification) = self.justification(header.clone())? {
            Ok(BlockStatus::Justified(justification))
        } else {
            Ok(BlockStatus::Present(header))
        }
    }

    fn best_block(&self) -> Result<B::Header, Self::Error> {
        let best_hash = self.best_hash();

        self.header_for_hash(best_hash)?
            .ok_or(Error::MissingHash(best_hash))
    }

    fn top_finalized(&self) -> Result<Justification<B::Header>, Self::Error> {
        let finalized_hash = self.finalized_hash();
        let header = self
            .header_for_hash(finalized_hash)?
            .ok_or(Error::MissingHash(finalized_hash))?;
        self.justification(header)?
            .ok_or(Error::MissingJustification(finalized_hash))
    }

    fn children(
        &self,
        id: <B::Header as Header>::Identifier,
    ) -> Result<Vec<B::Header>, Self::Error> {
        // This checks whether we have the block at all and the provided id is consistent.
        self.header(&id)?;
        Ok(self
            .backend
            .blockchain()
            .children(id.hash)?
            .into_iter()
            .map(|hash| self.header_for_hash(hash))
            .collect::<Result<Vec<Option<B::Header>>, BackendError>>()?
            .into_iter()
            .flatten()
            .collect())
    }
}
