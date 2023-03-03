use std::marker::PhantomData;

use log::warn;
use sp_runtime::traits::Block;

use crate::{
    abft::SignatureSet,
    crypto::Signature,
    data_io::{AlephData, AlephNetworkMessage},
    network::{data::Network, Data},
    Hasher, Recipient,
};

pub type LegacyNetworkData<B> =
    legacy_aleph_bft::NetworkData<Hasher, AlephData<B>, Signature, SignatureSet<Signature>>;

pub type CurrentNetworkData<B> =
    current_aleph_bft::NetworkData<Hasher, AlephData<B>, Signature, SignatureSet<Signature>>;

impl<B: Block> AlephNetworkMessage<B>
    for legacy_aleph_bft::NetworkData<Hasher, AlephData<B>, Signature, SignatureSet<Signature>>
{
    fn included_data(&self) -> Vec<AlephData<B>> {
        self.included_data()
    }
}

impl<B: Block> AlephNetworkMessage<B>
    for current_aleph_bft::NetworkData<Hasher, AlephData<B>, Signature, SignatureSet<Signature>>
{
    fn included_data(&self) -> Vec<AlephData<B>> {
        self.included_data()
    }
}

/// A wrapper needed only because of type system theoretical constraints. Sadness.
pub struct NetworkWrapper<D: Data, DN: Network<D>> {
    inner: DN,
    _phantom: PhantomData<D>,
}

impl<D: Data, DN: Network<D>> From<DN> for NetworkWrapper<D, DN> {
    fn from(inner: DN) -> Self {
        NetworkWrapper {
            inner,
            _phantom: PhantomData,
        }
    }
}

impl<D: Data, DN: Network<D>> NetworkWrapper<D, DN> {
    fn send<R>(&self, data: D, recipient: R)
    where
        R: Into<Recipient>,
    {
        if let Err(e) = self.inner.send(data, recipient.into()) {
            warn!(target: "aleph-network", "Error '{:?}' while sending an AlephBFT message to the network.", e);
        }
    }

    async fn next_event(&mut self) -> Option<D> {
        self.inner.next().await
    }
}

#[async_trait::async_trait]
impl<D: Data, DN: Network<D>> current_aleph_bft::Network<D> for NetworkWrapper<D, DN> {
    fn send(&self, data: D, recipient: current_aleph_bft::Recipient) {
        NetworkWrapper::send(self, data, recipient)
    }

    async fn next_event(&mut self) -> Option<D> {
        NetworkWrapper::next_event(self).await
    }
}

#[async_trait::async_trait]
impl<D: Data, DN: Network<D>> legacy_aleph_bft::Network<D> for NetworkWrapper<D, DN> {
    fn send(&self, data: D, recipient: legacy_aleph_bft::Recipient) {
        NetworkWrapper::send(self, data, recipient)
    }

    async fn next_event(&mut self) -> Option<D> {
        NetworkWrapper::next_event(self).await
    }
}
