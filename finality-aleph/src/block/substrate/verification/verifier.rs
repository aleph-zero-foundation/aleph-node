use std::fmt::{Display, Error as FmtError, Formatter};

use sp_runtime::RuntimeAppPublic;

use crate::{
    aleph_primitives::SessionAuthorityData, crypto::AuthorityVerifier,
    justification::AlephJustification, AuthorityId,
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
