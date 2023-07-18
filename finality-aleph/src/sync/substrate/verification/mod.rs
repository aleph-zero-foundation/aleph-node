use std::{
    fmt::{Debug, Display, Error as FmtError, Formatter},
    sync::Arc,
};

use parity_scale_codec::Encode;
use sc_client_api::HeaderBackend;
use sp_runtime::traits::Header as SubstrateHeader;

use crate::{
    aleph_primitives::{Block, BlockNumber, Header},
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
pub struct SubstrateFinalizationInfo<BE: HeaderBackend<Block>>(Arc<BE>);

impl<BE: HeaderBackend<Block>> SubstrateFinalizationInfo<BE> {
    pub fn new(client: Arc<BE>) -> Self {
        Self(client)
    }
}

impl<BE: HeaderBackend<Block>> FinalizationInfo for SubstrateFinalizationInfo<BE> {
    fn finalized_number(&self) -> BlockNumber {
        self.0.info().finalized_number
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
            Verification(e) => write!(f, "{e}"),
            Cache(e) => write!(f, "{e}"),
        }
    }
}

impl<AP, FS> Verifier<Justification> for VerifierCache<AP, FS, Header>
where
    AP: AuthorityProvider,
    FS: FinalizationInfo,
{
    type Error = VerificationError;

    fn verify(&mut self, justification: Justification) -> Result<Justification, Self::Error> {
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
