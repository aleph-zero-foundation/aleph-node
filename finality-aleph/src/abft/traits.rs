//! Implementations and definitions of traits used in legacy & current abft

use std::{cmp::Ordering, fmt::Debug, hash::Hash as StdHash, marker::PhantomData, pin::Pin};

use futures::{channel::oneshot, Future};
use network_clique::{SpawnHandleExt, SpawnHandleT};
use parity_scale_codec::{Decode, Encode};
use sc_service::SpawnTaskHandle;
use sp_runtime::traits::Hash as SpHash;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Wrapper<H: SpHash> {
    phantom: PhantomData<H>,
}

/// AlephBFT requires an order on hashes and `SpHash` does not have one, so we wrap it to add it.
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

impl<H: SpHash> Wrapper<H> {
    fn hash(s: &[u8]) -> OrdForHash<H::Output> {
        OrdForHash {
            inner: <H as SpHash>::hash(s),
        }
    }

    #[cfg(test)]
    pub fn random_hash() -> OrdForHash<H::Output> {
        use rand::distributions::{Alphanumeric, DistString};
        let string = Alphanumeric.sample_string(&mut rand::thread_rng(), 137);
        Self::hash(string.as_ref())
    }
}

impl<H: SpHash> current_aleph_bft::Hasher for Wrapper<H> {
    type Hash = OrdForHash<H::Output>;

    fn hash(s: &[u8]) -> Self::Hash {
        Wrapper::<H>::hash(s)
    }
}

impl<H: SpHash> legacy_aleph_bft::Hasher for Wrapper<H> {
    type Hash = OrdForHash<H::Output>;

    fn hash(s: &[u8]) -> Self::Hash {
        Wrapper::<H>::hash(s)
    }
}

/// A wrapper for spawning tasks in a way compatible with AlephBFT.
#[derive(Clone)]
pub struct SpawnHandle(SpawnTaskHandle);

impl SpawnHandle {
    pub fn spawn_essential_with_result(
        &self,
        name: &'static str,
        task: impl Future<Output = Result<(), ()>> + Send + 'static,
    ) -> Pin<Box<dyn Future<Output = Result<(), ()>> + Send>> {
        let (tx, rx) = oneshot::channel();
        let wrapped_task = async move {
            let result = task.await;
            let _ = tx.send(result);
        };
        let result = <Self as SpawnHandleExt>::spawn_essential(self, name, wrapped_task);
        let wrapped_result = async move {
            let main_result = result.await;
            if main_result.is_err() {
                return Err(());
            }
            let rx_result = rx.await;
            rx_result.unwrap_or(Err(()))
        };
        Box::pin(wrapped_result)
    }
}

impl From<SpawnTaskHandle> for SpawnHandle {
    fn from(sth: SpawnTaskHandle) -> Self {
        SpawnHandle(sth)
    }
}

impl SpawnHandleT for SpawnHandle {
    fn spawn(&self, name: &'static str, task: impl Future<Output = ()> + Send + 'static) {
        self.0.spawn(name, None, task)
    }
}

impl current_aleph_bft::SpawnHandle for SpawnHandle {
    fn spawn(&self, name: &'static str, task: impl Future<Output = ()> + Send + 'static) {
        SpawnHandleT::spawn(self, name, task)
    }

    fn spawn_essential(
        &self,
        name: &'static str,
        task: impl Future<Output = ()> + Send + 'static,
    ) -> current_aleph_bft::TaskHandle {
        SpawnHandleExt::spawn_essential(self, name, task)
    }
}

impl legacy_aleph_bft::SpawnHandle for SpawnHandle {
    fn spawn(&self, name: &'static str, task: impl Future<Output = ()> + Send + 'static) {
        SpawnHandleT::spawn(self, name, task)
    }

    fn spawn_essential(
        &self,
        name: &'static str,
        task: impl Future<Output = ()> + Send + 'static,
    ) -> legacy_aleph_bft::TaskHandle {
        SpawnHandleExt::spawn_essential(self, name, task)
    }
}
