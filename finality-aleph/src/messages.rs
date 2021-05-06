use crate::{hash::Hash, AuthorityId, AuthorityKeystore, AuthoritySignature, EpochId, UnitCoord};
use codec::{Decode, Encode, Error, Input, Output};
use log::debug;
use rush::{NodeCount, NodeIndex, PreUnit};
use sp_application_crypto::RuntimeAppPublic;

use sp_runtime::traits::Block;

#[derive(Debug, Encode, Decode, Clone)]
pub(crate) struct FullUnit<B: Block, H: Hash> {
    pub(crate) inner: PreUnit<H>,
    pub(crate) block_hash: B::Hash,
    pub(crate) epoch_id: EpochId,
}

#[derive(Debug, Clone)]
pub(crate) struct SignedUnit<B: Block, H: Hash> {
    pub(crate) unit: FullUnit<B, H>,
    signature: AuthoritySignature,
    // TODO: This *must* be changed ASAP to NodeIndex to reduce data size of packets.
    id: AuthorityId,
}
/// We use a custom implementation of Codec, which verifies the signature on Encode
impl<B: Block, H: Hash> Encode for SignedUnit<B, H> {
    fn size_hint(&self) -> usize {
        self.unit.size_hint() + self.signature.size_hint() + self.id.size_hint()
    }

    fn encode_to<W: Output + ?Sized>(&self, dest: &mut W) {
        self.unit.encode_to(dest);
        self.signature.encode_to(dest);
        self.id.encode_to(dest);
    }
}

impl<B: Block, H: Hash> Decode for SignedUnit<B, H> {
    fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
        let unit_size = <FullUnit<B, H> as Decode>::encoded_fixed_size()
            .ok_or(Error::from("FullUnit should be fixed size"))?;
        let mut unit = vec![0; unit_size];
        input.read(&mut unit)?;

        let signature = AuthoritySignature::decode(input)?;
        let id = AuthorityId::decode(input)?;
        if !id.verify(&unit, &signature) {
            return Err(Error::from("Bad signature"));
        }
        let unit = Decode::decode(&mut unit.as_slice())?;
        Ok(SignedUnit {
            unit,
            signature,
            id,
        })
    }
}

impl<B: Block, H: Hash> SignedUnit<B, H> {
    /// Verifies the unit's signature. The signature is verified on creation, so this should always
    /// return true, but the method can be used to check integrity.
    pub(crate) fn verify_unit_signature(&self) -> bool {
        if !self.id.verify(&self.unit.encode(), &self.signature) {
            debug!(target: "afa", "Bad signature in a unit from {:?}", self.unit.inner.creator());
            return false;
        }
        true
    }

    pub(crate) fn hash(&self, hashing: impl Fn(&[u8]) -> H) -> H {
        hashing(&self.unit.encode())
    }

    pub(crate) fn coord(&self) -> UnitCoord {
        (self.unit.inner.round(), self.unit.inner.creator()).into()
    }

    pub(crate) fn round(&self) -> usize {
        self.unit.inner.round()
    }

    pub(crate) fn creator(&self) -> NodeIndex {
        self.unit.inner.creator()
    }

    pub(crate) fn n_parents(&self) -> NodeCount {
        self.unit.inner.n_parents()
    }

    pub(crate) fn n_members(&self) -> NodeCount {
        self.unit.inner.n_members()
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

#[derive(Debug, Encode, Decode, Clone)]
pub(crate) struct ForkProof<B: Block, H: Hash> {
    pub(crate) u1: SignedUnit<B, H>,
    pub(crate) u2: SignedUnit<B, H>,
}

#[derive(Debug, Encode, Decode, Clone)]
pub(crate) struct Alert<B: Block, H: Hash> {
    pub(crate) sender: NodeIndex,
    pub(crate) forker: NodeIndex,
    pub(crate) proof: ForkProof<B, H>,
    pub(crate) legit_units: Vec<SignedUnit<B, H>>,
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
    /// Alert regarding forks,
    ForkAlert(Alert<B, H>),
}

/// The kind of message that is being sent.
#[derive(Debug, Encode, Decode, Clone)]
pub(crate) enum NetworkMessage<B: Block, H: Hash> {
    Consensus(ConsensusMessage<B, H>, EpochId),
}
