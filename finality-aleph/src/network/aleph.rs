use crate::{
    crypto::Signature,
    data_io::{AlephData, AlephNetworkMessage},
    network::DataNetwork,
    Hasher,
};
use aleph_bft::{Network as AlephNetwork, NetworkData as AlephNetworkData, SignatureSet};
use log::warn;
use sp_runtime::traits::Block;
use std::marker::PhantomData;

pub type NetworkData<B> =
    AlephNetworkData<Hasher, AlephData<B>, Signature, SignatureSet<Signature>>;

impl<B: Block> AlephNetworkMessage<B> for NetworkData<B> {
    fn included_data(&self) -> Vec<AlephData<B>> {
        self.included_data()
    }
}

/// A wrapper needed only because of type system theoretical constraints. Sadness.
pub struct NetworkWrapper<B: Block, ADN: DataNetwork<NetworkData<B>>> {
    inner: ADN,
    phantom: PhantomData<B>,
}

impl<B: Block, ADN: DataNetwork<NetworkData<B>>> From<ADN> for NetworkWrapper<B, ADN> {
    fn from(inner: ADN) -> Self {
        NetworkWrapper {
            inner,
            phantom: PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<B: Block, ADN: DataNetwork<NetworkData<B>>>
    AlephNetwork<Hasher, AlephData<B>, Signature, SignatureSet<Signature>>
    for NetworkWrapper<B, ADN>
{
    fn send(&self, data: NetworkData<B>, recipient: aleph_bft::Recipient) {
        if self.inner.send(data, recipient).is_err() {
            warn!(target: "aleph-network", "Error sending an AlephBFT message to the network.");
        }
    }

    async fn next_event(&mut self) -> Option<NetworkData<B>> {
        self.inner.next().await
    }
}
