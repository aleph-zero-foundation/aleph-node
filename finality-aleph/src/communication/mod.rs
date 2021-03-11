mod gossip;
pub(crate) mod peer;

use rush::{nodes::NodeIndex, EpochId, Round};
use sp_runtime::traits::{Block, Hash, Header};

pub const ALEPH_AUTHORITIES_KEY: &[u8] = b":aleph_authorities";

pub(crate) fn multicast_topic<B: Block>(round: Round, epoch: EpochId) -> B::Hash {
    <<B::Header as Header>::Hashing as Hash>::hash(format!("{}-{}", round, epoch.0).as_bytes())
}

pub(crate) fn index_topic<B: Block>(index: NodeIndex) -> B::Hash {
    <<B::Header as Header>::Hashing as Hash>::hash(format!("{}", index.0).as_bytes())
}
