//! A set of abstractions for dealing with `ReliableMulticast` in a more testable
//! and modular way.
//!
//! We expose the following traits:
//! - `Multicast`: mimicking the interface of `aleph_bft::ReliableMulticast`
//! - `Multisigned`: abstracting away the `aleph_bft::Multisigned` capabilities as a trait.

use aleph_bft::{
    rmc::ReliableMulticast, MultiKeychain, Multisigned as MultisignedStruct, Signable,
    UncheckedSigned,
};
use codec::{Codec, Decode, Encode};
use std::{fmt::Debug, hash::Hash as StdHash};

/// A convenience trait for gathering all of the desired hash characteristics.
pub trait Hash: AsRef<[u8]> + StdHash + Eq + Clone + Codec + Debug + Send + Sync {}

impl<T: AsRef<[u8]> + StdHash + Eq + Clone + Codec + Debug + Send + Sync> Hash for T {}

/// A wrapper allowing block hashes to be signed.
#[derive(PartialEq, Eq, StdHash, Clone, Debug, Default, Encode, Decode)]
pub struct SignableHash<H: Hash> {
    hash: H,
}

impl<H: Hash> SignableHash<H> {
    pub fn new(hash: H) -> Self {
        Self { hash }
    }
}

impl<H: Hash> Signable for SignableHash<H> {
    type Hash = H;
    fn hash(&self) -> Self::Hash {
        self.hash.clone()
    }
}

/// Anything that exposes the same interface as `aleph_bft::Multisigned` struct.
pub trait Multisigned<'a, S: Signable, MK: MultiKeychain> {
    fn as_signable(&self) -> &S;
    fn into_unchecked(self) -> UncheckedSigned<S, MK::PartialMultisignature>;
}

impl<'a, H: Hash, MK: MultiKeychain> Multisigned<'a, SignableHash<H>, MK>
    for MultisignedStruct<'a, SignableHash<H>, MK>
{
    fn as_signable(&self) -> &SignableHash<H> {
        self.as_signable()
    }

    fn into_unchecked(self) -> UncheckedSigned<SignableHash<H>, MK::PartialMultisignature> {
        self.into_unchecked()
    }
}

/// Anything that exposes the same interface as `aleph_bft::ReliableMulticast`.
///
/// The trait defines an associated type: `Signed`. For `ReliableMulticast`, this is simply
/// `aleph_bft::Multisigned` but the mocks are free to use the simplest matching implementation.
#[async_trait::async_trait]
pub trait Multicast<H: Hash, S: Signable>: Send + Sync {
    type Signed;
    async fn start_multicast(&mut self, signable: S);
    fn get_multisigned(&self, signable: &S) -> Option<Self::Signed>;
    async fn next_multisigned_hash(&mut self) -> Self::Signed;
}

#[async_trait::async_trait]
impl<'a, H: Hash, MK: MultiKeychain> Multicast<H, SignableHash<H>>
    for ReliableMulticast<'a, SignableHash<H>, MK>
{
    type Signed = MultisignedStruct<'a, SignableHash<H>, MK>;

    async fn start_multicast(&mut self, hash: SignableHash<H>) {
        self.start_rmc(hash).await;
    }

    fn get_multisigned(
        &self,
        hash: &SignableHash<H>,
    ) -> Option<MultisignedStruct<'a, SignableHash<H>, MK>> {
        self.get_multisigned(hash)
    }

    async fn next_multisigned_hash(&mut self) -> MultisignedStruct<'a, SignableHash<H>, MK> {
        self.next_multisigned_hash().await
    }
}
