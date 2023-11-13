use std::{
    fmt::{Debug, Display},
    hash::Hash as StdHash,
};

use aleph_bft_rmc::{Message as RmcMessage, Signable};
use aleph_bft_types::Recipient;
use parity_scale_codec::{Codec, Decode, Encode};

mod aggregator;

pub use crate::aggregator::{BlockSignatureAggregator, IO};

pub type RmcNetworkData<H, S, SS> = RmcMessage<SignableHash<H>, S, SS>;

/// A convenience trait for gathering all of the desired hash characteristics.
pub trait Hash: AsRef<[u8]> + StdHash + Eq + Clone + Codec + Debug + Display + Send + Sync {}

impl<T: AsRef<[u8]> + StdHash + Eq + Clone + Codec + Debug + Display + Send + Sync> Hash for T {}

/// A wrapper allowing block hashes to be signed.
#[derive(PartialEq, Eq, StdHash, Clone, Debug, Default, Encode, Decode)]
pub struct SignableHash<H: Hash> {
    hash: H,
}

impl<H: Hash> SignableHash<H> {
    pub fn new(hash: H) -> Self {
        Self { hash }
    }

    pub fn get_hash(&self) -> H {
        self.hash.clone()
    }
}

impl<H: Hash> Signable for SignableHash<H> {
    type Hash = H;
    fn hash(&self) -> Self::Hash {
        self.hash.clone()
    }
}

#[derive(Debug)]
pub enum NetworkError {
    SendFail,
}

#[async_trait::async_trait]
pub trait ProtocolSink<D>: Send + Sync {
    async fn next(&mut self) -> Option<D>;
    fn send(&self, data: D, recipient: Recipient) -> Result<(), NetworkError>;
}
