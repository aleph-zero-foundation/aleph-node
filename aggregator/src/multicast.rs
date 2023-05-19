//! A set of abstractions for dealing with `ReliableMulticast` in a more testable
//! and modular way.
//!
//! We expose the `Multicast` trait, mimicking the interface of `aleph_bft::ReliableMulticast`

use std::{
    fmt::{Debug, Display},
    hash::Hash as StdHash,
};

use aleph_bft_rmc::{MultiKeychain, ReliableMulticast, Signable, Signature};
use parity_scale_codec::{Codec, Decode, Encode};

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

/// Anything that exposes the same interface as `aleph_bft::ReliableMulticast`.
///
/// The trait defines an associated type: `Signed`. For `ReliableMulticast`, this is simply
/// `aleph_bft::Multisigned` but the mocks are free to use the simplest matching implementation.
#[async_trait::async_trait]
pub trait Multicast<H: Hash, PMS>: Send + Sync {
    async fn start_multicast(&mut self, signable: SignableHash<H>);
    async fn next_signed_pair(&mut self) -> (H, PMS);
}

#[async_trait::async_trait]
impl<'a, H: Hash, MK: MultiKeychain<PartialMultisignature = SS>, SS: Signature + Send + Sync>
    Multicast<H, SS> for ReliableMulticast<'a, SignableHash<H>, MK>
{
    async fn start_multicast(&mut self, hash: SignableHash<H>) {
        self.start_rmc(hash).await;
    }

    async fn next_signed_pair(&mut self) -> (H, SS) {
        let ms = self.next_multisigned_hash().await.into_unchecked();
        (ms.as_signable().get_hash(), ms.signature())
    }
}
