use std::{
    fmt::{Display, Error as FmtError, Formatter},
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
            verification::{
                cache::{CacheError, VerifierCache},
                verifier::SessionVerificationError,
            },
            Justification,
        },
        Verifier,
    },
};

mod cache;
mod verifier;

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

impl<H, AP, FS> Verifier<Justification<H>> for VerifierCache<AP, FS>
where
    H: SubstrateHeader<Number = BlockNumber>,
    AP: AuthorityProvider,
    FS: FinalizationInfo,
{
    type Error = VerificationError;

    fn verify(&mut self, justification: Justification<H>) -> Result<Justification<H>, Self::Error> {
        let header = &justification.header;
        let verifier = self.get(*header.number())?;
        verifier.verify_bytes(&justification.raw_justification, header.hash().encode())?;
        Ok(justification)
    }
}
