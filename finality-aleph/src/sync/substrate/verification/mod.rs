use std::{
    fmt::{Debug, Display, Error as FmtError, Formatter},
    marker::PhantomData,
    sync::Arc,
};

use aleph_primitives::BlockNumber;
use codec::Encode;
use sc_client_api::HeaderBackend;
use sp_runtime::traits::{Block as BlockT, Header as SubstrateHeader};

use crate::{
    session_map::AuthorityProvider,
    sync::{
        substrate::{
            verification::{cache::CacheError, verifier::SessionVerificationError},
            InnerJustification, Justification,
        },
        Verifier,
    },
};

mod cache;
mod verifier;

pub use cache::VerifierCache;
pub use verifier::SessionVerifier;

/// Supplies finalized number. Will be unified together with other traits we used in A0-1839.
pub trait FinalizationInfo {
    fn finalized_number(&self) -> BlockNumber;
}

/// Substrate specific implementation of `FinalizationInfo`
pub struct SubstrateFinalizationInfo<B, BE>
where
    BE: HeaderBackend<B>,
    B: BlockT,
    B::Header: SubstrateHeader<Number = BlockNumber>,
{
    client: Arc<BE>,
    _phantom: PhantomData<B>,
}

impl<B, BE> SubstrateFinalizationInfo<B, BE>
where
    BE: HeaderBackend<B>,
    B: BlockT,
    B::Header: SubstrateHeader<Number = BlockNumber>,
{
    pub fn new(client: Arc<BE>) -> Self {
        Self {
            client,
            _phantom: PhantomData,
        }
    }
}

impl<B, BE> FinalizationInfo for SubstrateFinalizationInfo<B, BE>
where
    BE: HeaderBackend<B>,
    B: BlockT,
    B::Header: SubstrateHeader<Number = BlockNumber>,
{
    fn finalized_number(&self) -> BlockNumber {
        self.client.info().finalized_number
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum VerificationError {
    Verification(SessionVerificationError),
    Cache(CacheError),
}

impl From<SessionVerificationError> for VerificationError {
    fn from(e: SessionVerificationError) -> Self {
        VerificationError::Verification(e)
    }
}

impl From<CacheError> for VerificationError {
    fn from(e: CacheError) -> Self {
        VerificationError::Cache(e)
    }
}

impl Display for VerificationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use VerificationError::*;
        match self {
            Verification(e) => write!(f, "{}", e),
            Cache(e) => write!(f, "{}", e),
        }
    }
}

impl<AP, FS, H> Verifier<Justification<H>> for VerifierCache<AP, FS, H>
where
    AP: AuthorityProvider,
    FS: FinalizationInfo,
    H: SubstrateHeader<Number = BlockNumber>,
{
    type Error = VerificationError;

    fn verify(&mut self, justification: Justification<H>) -> Result<Justification<H>, Self::Error> {
        let header = &justification.header;
        match &justification.inner_justification {
            InnerJustification::AlephJustification(aleph_justification) => {
                let verifier = self.get(*header.number())?;
                verifier.verify_bytes(aleph_justification, header.hash().encode())?;
                Ok(justification)
            }
            InnerJustification::Genesis => match header == self.genesis_header() {
                true => Ok(justification),
                false => Err(Self::Error::Cache(CacheError::BadGenesisHeader)),
            },
        }
    }
}
