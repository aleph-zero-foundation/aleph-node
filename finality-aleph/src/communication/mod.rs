mod gossip;
pub(super) mod peer;

use crate::{
    temp::{EpochId, NodeIndex, Round, Unit, UnitCoord},
    AuthorityId, AuthoritySignature,
};
use codec::{Decode, Encode};
use log::debug;
use sp_application_crypto::RuntimeAppPublic;
use sp_runtime::{
    traits::{Block, Hash, Header},
    ConsensusEngineId,
};

/// Name of the notifications protocol used by Aleph Zero. This is how messages
/// are subscribed to to ensure that we are gossiping and communicating with our
/// own network.
pub const ALEPH_PROTOCOL_NAME: &str = "/cardinals/aleph/1";

pub const ALEPH_ENGINE_ID: ConsensusEngineId = *b"ALPH";

pub const ALEPH_AUTHORITIES_KEY: &[u8] = b":aleph_authorities";

pub(crate) fn multicast_topic<B: Block>(round: Round, epoch: EpochId) -> B::Hash {
    <<B::Header as Header>::Hashing as Hash>::hash(format!("{}-{}", round, epoch).as_bytes())
}

pub(crate) fn index_topic<B: Block>(index: NodeIndex) -> B::Hash {
    <<B::Header as Header>::Hashing as Hash>::hash(format!("{}", index).as_bytes())
}
