mod gossip;
pub mod network;
pub(crate) mod peer;

use rush::EpochId;
use sp_runtime::traits::{Block, Hash, Header};

pub const ALEPH_AUTHORITIES_KEY: &[u8] = b":aleph_authorities";

pub(crate) fn epoch_topic<B: Block>(epoch: EpochId) -> B::Hash {
    <<B::Header as Header>::Hashing as Hash>::hash(format!("epoch-{}", epoch.0).as_bytes())
}

pub(crate) fn request_topic<B: Block>() -> B::Hash {
    <<B::Header as Header>::Hashing as Hash>::hash("request".as_bytes())
}
