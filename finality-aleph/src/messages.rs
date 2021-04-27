use crate::{hash::Hash, AuthorityId, AuthorityKeystore, AuthoritySignature, EpochId, UnitCoord};
use codec::{Decode, Encode};
use log::debug;
use rush::PreUnit;
use sp_application_crypto::RuntimeAppPublic;

use sp_runtime::traits::Block;

#[derive(Debug, Encode, Decode, Clone)]
pub(crate) struct FullUnit<B: Block, H: Hash> {
    pub(crate) inner: PreUnit<H>,
    pub(crate) block_hash: B::Hash,
    pub(crate) epoch_id: EpochId,
}

#[derive(Debug, Encode, Decode, Clone)]
pub(crate) struct SignedUnit<B: Block, H: Hash> {
    pub(crate) unit: FullUnit<B, H>,
    signature: AuthoritySignature,
    // TODO: This *must* be changed ASAP to NodeIndex to reduce data size of packets.
    id: AuthorityId,
}

//TODO: refactor the below, not sure what the buffors are for
impl<B: Block, H: Hash> SignedUnit<B, H> {
    /// Encodes the unit with a buffer vector.
    pub(crate) fn encode_unit_with_buffer(&self, buf: &mut Vec<u8>) {
        buf.clear();
        self.unit.encode_to(buf);
    }

    /// Verifies the unit's signature with a buffer.
    pub(crate) fn verify_unit_signature_with_buffer(&self, buf: &mut Vec<u8>) -> bool {
        self.encode_unit_with_buffer(buf);

        let valid = self.id.verify(&buf, &self.signature);
        if !valid {
            debug!(target: "afa", "Bad signature message from {:?}", self.unit.inner.creator());
        }

        valid
    }

    /// Verifies the unit's signature.
    pub(crate) fn verify_unit_signature(&self) -> bool {
        self.verify_unit_signature_with_buffer(&mut Vec::new())
    }

    pub(crate) fn hash(&self, hashing: impl Fn(&[u8]) -> H) -> H {
        hashing(&self.unit.encode())
    }
}

pub(crate) fn sign_unit<B: Block, H: Hash>(
    auth_crypto_store: &AuthorityKeystore,
    unit: FullUnit<B, H>,
) -> SignedUnit<B, H> {
    let encoded = unit.encode();
    let signature = auth_crypto_store.sign(&encoded[..]);

    SignedUnit {
        unit,
        signature,
        id: auth_crypto_store.authority_id.clone(),
    }
}

/// The kind of message that is being sent.
#[derive(Debug, Encode, Decode, Clone)]
pub(crate) enum ConsensusMessage<B: Block, H: Hash> {
    /// Fo disseminating newly created units.
    NewUnit(SignedUnit<B, H>),
    /// Request for a unit by its coord.
    RequestCoord(UnitCoord),
    /// Response to a request by coord.
    ResponseCoord(SignedUnit<B, H>),
    /// Request for the full list of parents of a unit.
    RequestParents(H),
    /// Response to a request for a full list of parents.
    ResponseParents(H, Vec<SignedUnit<B, H>>),
}

/// The kind of message that is being sent.
#[derive(Debug, Encode, Decode, Clone)]
pub(crate) enum NetworkMessage<B: Block, H: Hash> {
    Consensus(ConsensusMessage<B, H>, EpochId),
}
