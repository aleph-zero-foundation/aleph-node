use codec::{Decode, Encode};
use rush::Hasher;
use sp_runtime::traits::Hash;
use std::{cmp::Ordering, fmt::Debug, hash::Hash as StdHash, marker::PhantomData};

#[derive(Debug, PartialEq, Eq, Clone, Copy, StdHash, Encode, Decode)]
pub struct OrdForHash<O: Eq + Copy + Clone + Send + Debug + StdHash + Encode + Decode + AsRef<[u8]>>
{
    inner: O,
}

impl<O: Eq + Copy + Clone + Send + Sync + Debug + StdHash + Encode + Decode + AsRef<[u8]>>
    PartialOrd for OrdForHash<O>
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<O: Eq + Copy + Clone + Send + Sync + Debug + StdHash + Encode + Decode + AsRef<[u8]>> Ord
    for OrdForHash<O>
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.inner.as_ref().cmp(other.inner.as_ref())
    }
}

impl<O: Eq + Copy + Clone + Send + Sync + Debug + StdHash + Encode + Decode + AsRef<[u8]>>
    AsRef<[u8]> for OrdForHash<O>
{
    fn as_ref(&self) -> &[u8] {
        self.inner.as_ref()
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Wrapper<H: Hash> {
    phantom: PhantomData<H>,
}

impl<H: Hash> Hasher for Wrapper<H> {
    type Hash = OrdForHash<H::Output>;

    fn hash(s: &[u8]) -> Self::Hash {
        Self::Hash {
            inner: <H as Hash>::hash(s),
        }
    }
}
