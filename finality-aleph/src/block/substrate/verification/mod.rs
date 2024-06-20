use std::{
    fmt::{Debug, Display, Error as FmtError, Formatter},
    sync::Arc,
};

use hex::ToHex;
use sc_client_api::HeaderBackend;
use sc_consensus_aura::standalone::{PreDigestLookupError, SealVerificationError};
use sp_consensus_slots::Slot;

use crate::{
    aleph_primitives::{AccountId, AuraId, Block, BlockNumber, Header},
    block::{
        substrate::{
            verification::{cache::CacheError, verifier::SessionVerificationError},
            FinalizationInfo,
        },
        EquivocationProof as EquivocationProofT, Header as HeaderT,
    },
};

mod cache;
mod verifier;

pub use cache::VerifierCache;

/// Substrate specific implementation of `FinalizationInfo`
pub struct SubstrateFinalizationInfo<BE: HeaderBackend<Block>>(Arc<BE>);

impl<BE: HeaderBackend<Block>> Clone for SubstrateFinalizationInfo<BE> {
    fn clone(&self) -> Self {
        SubstrateFinalizationInfo(self.0.clone())
    }
}

impl<BE: HeaderBackend<Block>> SubstrateFinalizationInfo<BE> {
    pub fn new(client: Arc<BE>) -> Self {
        Self(client)
    }
}

impl<BE: HeaderBackend<Block> + 'static> FinalizationInfo for SubstrateFinalizationInfo<BE> {
    fn finalized_number(&self) -> BlockNumber {
        self.0.info().finalized_number
    }
}

#[derive(Debug)]
pub enum HeaderVerificationError {
    PreDigestLookupError(PreDigestLookupError),
    HeaderTooNew(Slot),
    IncorrectGenesis,
    MissingSeal,
    IncorrectSeal,
    MissingAuthorityData,
    IncorrectAuthority,
}

impl Display for HeaderVerificationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use HeaderVerificationError::*;
        match self {
            PreDigestLookupError(e) => write!(f, "pre digest lookup error, {e}"),
            HeaderTooNew(slot) => write!(f, "slot {slot} too far in the future"),
            IncorrectGenesis => write!(f, "incorrect genesis header"),
            MissingSeal => write!(f, "missing seal"),
            IncorrectSeal => write!(f, "incorrect seal"),
            MissingAuthorityData => write!(f, "missing authority data"),
            IncorrectAuthority => write!(f, "incorrect authority"),
        }
    }
}

impl<Header> From<SealVerificationError<Header>> for HeaderVerificationError {
    fn from(value: SealVerificationError<Header>) -> Self {
        match value {
            SealVerificationError::Deferred(_, slot) => Self::HeaderTooNew(slot),
            SealVerificationError::Unsealed => Self::MissingSeal,
            SealVerificationError::BadSeal => Self::IncorrectSeal,
            SealVerificationError::BadSignature => Self::IncorrectAuthority,
            SealVerificationError::SlotAuthorNotFound => Self::MissingAuthorityData,
            SealVerificationError::InvalidPreDigest(err) => Self::PreDigestLookupError(err),
        }
    }
}

pub struct EquivocationProof {
    header_a: Header,
    header_b: Header,
    author: AuraId,
    account_id: Option<AccountId>,
    are_we_equivocating: bool,
}

impl EquivocationProofT for EquivocationProof {
    fn are_we_equivocating(&self) -> bool {
        self.are_we_equivocating
    }
}

impl Display for EquivocationProof {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        match &self.account_id {
            Some(account_id) => write!(
                f,
                "account ID: {}, author: 0x{}, first header: {}, second header {}",
                account_id,
                self.author.encode_hex::<String>(),
                self.header_a.id(),
                self.header_b.id()
            ),
            None => write!(
                f,
                "author: 0x{}, first header: {}, second header {}; check the account ID by hand",
                self.author.encode_hex::<String>(),
                self.header_a.id(),
                self.header_b.id()
            ),
        }
    }
}

#[derive(Debug)]
pub enum VerificationError {
    Verification(SessionVerificationError),
    Cache(CacheError),
    HeaderVerification(HeaderVerificationError),
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

impl From<HeaderVerificationError> for VerificationError {
    fn from(value: HeaderVerificationError) -> Self {
        Self::HeaderVerification(value)
    }
}

impl Display for VerificationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use VerificationError::*;
        match self {
            Verification(e) => write!(f, "{e}"),
            Cache(e) => write!(f, "{e}"),
            HeaderVerification(e) => write!(f, "{e}"),
        }
    }
}
