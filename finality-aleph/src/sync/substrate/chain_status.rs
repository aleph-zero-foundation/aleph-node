use std::{
    fmt::{Display, Error as FmtError, Formatter},
    sync::Arc,
};

use log::warn;
use sc_client_api::{blockchain::HeaderBackend, Backend as _};
use sc_service::TFullBackend;
use sp_blockchain::{Backend as _, Error as BackendError, Info};
use sp_runtime::traits::{Block as SubstrateBlock, Header as SubstrateHeader};

use crate::{
    aleph_primitives::{
        Block as AlephBlock, BlockNumber, Hash as AlephHash, Header as AlephHeader, ALEPH_ENGINE_ID,
    },
    justification::backwards_compatible_decode,
    sync::{
        substrate::{BlockId, Justification, SubstrateSyncBlock},
        BlockIdFor, BlockStatus, ChainStatus, LOG_TARGET,
    },
};

/// What can go wrong when checking chain status
#[derive(Debug)]
pub enum Error {
    MissingHash(AlephHash),
    MissingBody(AlephHash),
    MissingJustification(AlephHash),
    Backend(BackendError),
    MismatchedId,
    NoGenesisBlock,
}

impl Display for Error {
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
            MissingBody(hash) => {
                write!(
                    f,
                    "data availability problem: no block body for existing hash {:?}",
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

impl From<BackendError> for Error {
    fn from(value: BackendError) -> Self {
        Error::Backend(value)
    }
}

/// Substrate implementation of ChainStatus trait
#[derive(Clone)]
pub struct SubstrateChainStatus {
    backend: Arc<TFullBackend<AlephBlock>>,
    genesis_header: AlephHeader,
}

impl SubstrateChainStatus {
    pub fn new(backend: Arc<TFullBackend<AlephBlock>>) -> Result<Self, Error> {
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

    fn info(&self) -> Info<AlephBlock> {
        self.backend.blockchain().info()
    }

    fn hash_for_number(&self, number: BlockNumber) -> Result<Option<AlephHash>, BackendError> {
        self.backend.blockchain().hash(number)
    }

    fn header_for_hash(&self, hash: AlephHash) -> Result<Option<AlephHeader>, BackendError> {
        self.backend.blockchain().header(hash)
    }

    fn body_for_hash(
        &self,
        hash: AlephHash,
    ) -> Result<Option<Vec<<AlephBlock as SubstrateBlock>::Extrinsic>>, BackendError> {
        self.backend.blockchain().body(hash)
    }

    fn indexed_body_for_hash(&self, hash: AlephHash) -> Result<Option<Vec<Vec<u8>>>, BackendError> {
        self.backend.blockchain().block_indexed_body(hash)
    }

    fn header(
        &self,
        id: &BlockIdFor<Justification<AlephHeader>>,
    ) -> Result<Option<AlephHeader>, Error> {
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
        header: AlephHeader,
    ) -> Result<Option<Justification<AlephHeader>>, BackendError> {
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

    fn best_hash(&self) -> AlephHash {
        self.info().best_hash
    }

    fn finalized_hash(&self) -> AlephHash {
        self.info().finalized_hash
    }
}

impl ChainStatus<SubstrateSyncBlock, Justification<AlephHeader>> for SubstrateChainStatus {
    type Error = Error;

    fn finalized_at(
        &self,
        number: BlockNumber,
    ) -> Result<Option<Justification<AlephHeader>>, Self::Error> {
        let id = match self.hash_for_number(number)? {
            Some(hash) => BlockId { hash, number },
            None => return Ok(None),
        };
        match self.status_of(id)? {
            BlockStatus::Justified(justification) => Ok(Some(justification)),
            _ => Ok(None),
        }
    }

    fn block(
        &self,
        id: BlockIdFor<Justification<AlephHeader>>,
    ) -> Result<Option<SubstrateSyncBlock>, Self::Error> {
        let header = match self.header(&id)? {
            Some(header) => header,
            None => return Ok(None),
        };
        let body = match self.body_for_hash(id.hash)? {
            Some(body) => body,
            None => return Err(Error::MissingBody(id.hash)),
        };
        let indexed_body = self.indexed_body_for_hash(id.hash)?;
        Ok(Some(SubstrateSyncBlock {
            inner: AlephBlock::new(header, body),
            indexed_body,
        }))
    }

    fn status_of(
        &self,
        id: BlockIdFor<Justification<AlephHeader>>,
    ) -> Result<BlockStatus<Justification<AlephHeader>>, Self::Error> {
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

    fn best_block(&self) -> Result<AlephHeader, Self::Error> {
        let best_hash = self.best_hash();

        self.header_for_hash(best_hash)?
            .ok_or(Error::MissingHash(best_hash))
    }

    fn top_finalized(&self) -> Result<Justification<AlephHeader>, Self::Error> {
        let finalized_hash = self.finalized_hash();
        let header = self
            .header_for_hash(finalized_hash)?
            .ok_or(Error::MissingHash(finalized_hash))?;
        self.justification(header)?
            .ok_or(Error::MissingJustification(finalized_hash))
    }

    fn children(
        &self,
        id: BlockIdFor<Justification<AlephHeader>>,
    ) -> Result<Vec<AlephHeader>, Self::Error> {
        // This checks whether we have the block at all and the provided id is consistent.
        self.header(&id)?;
        Ok(self
            .backend
            .blockchain()
            .children(id.hash)?
            .into_iter()
            .map(|hash| self.header_for_hash(hash))
            .collect::<Result<Vec<Option<AlephHeader>>, BackendError>>()?
            .into_iter()
            .flatten()
            .collect())
    }
}
