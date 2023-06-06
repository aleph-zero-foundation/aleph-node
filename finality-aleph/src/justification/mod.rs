use parity_scale_codec::{Decode, Encode};
use sp_runtime::Justification;

use crate::{
    abft::SignatureSet,
    aleph_primitives::{AuthoritySignature, ALEPH_ENGINE_ID},
    crypto::Signature,
};

mod compatibility;

pub use compatibility::{backwards_compatible_decode, versioned_encode, Error as DecodeError};

const LOG_TARGET: &str = "aleph-justification";

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
