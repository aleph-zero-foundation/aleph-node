use std::{marker::PhantomData, time::Duration};

use aleph_primitives::{AuthoritySignature, BlockNumber, ALEPH_ENGINE_ID};
use codec::{Decode, Encode};

use crate::{crypto::Signature, BlockIdentifier, IdentifierFor, SessionId};

mod compatibility;
mod handler;
mod requester;
mod scheduler;

pub use compatibility::{backwards_compatible_decode, versioned_encode, Error as DecodeError};
pub use handler::JustificationHandler;
pub use scheduler::{
    JustificationRequestScheduler, JustificationRequestSchedulerImpl, SchedulerActions,
};
use sp_runtime::Justification;

use crate::abft::SignatureSet;

/// A proof of block finality, currently in the form of a sufficiently long list of signatures or a
/// sudo signature of a block for emergency finalization.
#[derive(Clone, Encode, Decode, Debug, PartialEq, Eq)]
pub enum AlephJustification {
    CommitteeMultisignature(SignatureSet<Signature>),
    EmergencySignature(AuthoritySignature),
}

impl From<AlephJustification> for Justification {
    fn from(val: AlephJustification) -> Self {
        (ALEPH_ENGINE_ID, versioned_encode(val))
    }
}

pub trait Verifier<BI: BlockIdentifier> {
    fn verify(&self, justification: &AlephJustification, block_id: &BI) -> bool;
}

pub struct SessionInfo<BI: BlockIdentifier, V: Verifier<BI>> {
    pub current_session: SessionId,
    pub last_block_height: BlockNumber,
    pub verifier: Option<V>,
    _phantom: PhantomData<BI>,
}

impl<BI: BlockIdentifier, V: Verifier<BI>> SessionInfo<BI, V> {
    pub fn new(
        current_session: SessionId,
        last_block_height: BlockNumber,
        verifier: Option<V>,
    ) -> Self {
        Self {
            current_session,
            last_block_height,
            verifier,
            _phantom: PhantomData,
        }
    }
}

/// Returns `SessionInfo` for the session regarding block with no. `number`.
#[async_trait::async_trait]
pub trait SessionInfoProvider<BI: BlockIdentifier, V: Verifier<BI>> {
    async fn for_block_num(&self, number: BlockNumber) -> SessionInfo<BI, V>;
}

/// A notification for sending justifications over the network.
#[derive(Clone)]
pub struct JustificationNotification<BI: BlockIdentifier> {
    /// The justification itself.
    pub justification: AlephJustification,
    /// The ID of the finalized block.
    pub block_id: BI,
}

pub type JustificationNotificationFor<B> = JustificationNotification<IdentifierFor<B>>;

#[derive(Clone)]
pub struct JustificationHandlerConfig {
    /// How long should we wait when the session verifier is not yet available.
    verifier_timeout: Duration,
    /// How long should we wait for any notification.
    notification_timeout: Duration,
}

impl Default for JustificationHandlerConfig {
    fn default() -> Self {
        Self {
            verifier_timeout: Duration::from_millis(500),
            // request justifications slightly more frequently than they're created
            notification_timeout: Duration::from_millis(800),
        }
    }
}

#[cfg(test)]
impl JustificationHandlerConfig {
    pub fn new(verifier_timeout: Duration, notification_timeout: Duration) -> Self {
        Self {
            verifier_timeout,
            notification_timeout,
        }
    }
}
