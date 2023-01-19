use std::fmt::{Display, Error as FmtError, Formatter};

use aleph_primitives::SessionAuthorityData;
use codec::Encode;
use log::warn;
use sp_runtime::{traits::Block as BlockT, RuntimeAppPublic};

use crate::{
    crypto::AuthorityVerifier,
    justification::{AlephJustification, Verifier as LegacyVerifier},
    AuthorityId,
};

/// A justification verifier within a single session.
#[derive(Clone, PartialEq, Debug)]
pub struct SessionVerifier {
    authority_verifier: AuthorityVerifier,
    emergency_signer: Option<AuthorityId>,
}

impl From<SessionAuthorityData> for SessionVerifier {
    fn from(authority_data: SessionAuthorityData) -> Self {
        SessionVerifier {
            authority_verifier: AuthorityVerifier::new(authority_data.authorities().to_vec()),
            emergency_signer: authority_data.emergency_finalizer().clone(),
        }
    }
}

/// Ways in which a justification can be wrong.
#[derive(Debug, PartialEq, Eq)]
pub enum SessionVerificationError {
    BadMultisignature,
    BadEmergencySignature,
    NoEmergencySigner,
}

impl Display for SessionVerificationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use SessionVerificationError::*;
        match self {
            BadMultisignature => write!(f, "bad multisignature"),
            BadEmergencySignature => write!(f, "bad emergency signature"),
            NoEmergencySigner => write!(f, "no emergency signer defined"),
        }
    }
}

impl SessionVerifier {
    /// Verifies the correctness of a justification for supplied bytes.
    pub fn verify_bytes(
        &self,
        justification: &AlephJustification,
        bytes: Vec<u8>,
    ) -> Result<(), SessionVerificationError> {
        use AlephJustification::*;
        use SessionVerificationError::*;
        match justification {
            CommitteeMultisignature(multisignature) => {
                match self.authority_verifier.is_complete(&bytes, multisignature) {
                    true => Ok(()),
                    false => Err(BadMultisignature),
                }
            }
            EmergencySignature(signature) => match self
                .emergency_signer
                .as_ref()
                .ok_or(NoEmergencySigner)?
                .verify(&bytes, signature)
            {
                true => Ok(()),
                false => Err(BadEmergencySignature),
            },
        }
    }
}

// This shouldn't be necessary after we remove the legacy justification sync. Then we can also
// rewrite the implementation above and make it simpler.
impl<B: BlockT> LegacyVerifier<B> for SessionVerifier {
    fn verify(&self, justification: &AlephJustification, hash: B::Hash) -> bool {
        match self.verify_bytes(justification, hash.encode()) {
            Ok(()) => true,
            Err(e) => {
                warn!(target: "aleph-justification", "Bad justification for block {:?}: {}", hash, e);
                false
            }
        }
    }
}
