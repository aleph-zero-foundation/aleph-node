// All this should be removed after the old network is no longer in use.
// In particular I use the above to avoid worrying about the code duplication below.
// It cannot easily be avoided, because it's kinda hard to make AlephNetwork and RmcNetwork
// implement appropriate DataNetworks.
use crate::{
    network::{AlephNetwork, RmcNetwork},
    new_network::{AlephNetworkData, DataNetwork, RmcNetworkData, SendError},
};
use aleph_bft::{Network, Recipient};
use sp_runtime::traits::Block;

pub struct SplicedAlephNetwork<B: Block, DN: DataNetwork<AlephNetworkData<B>>> {
    new_aleph_network: DN,
    old_aleph_network: AlephNetwork<B>,
}

impl<B: Block, DN: DataNetwork<AlephNetworkData<B>>> SplicedAlephNetwork<B, DN> {
    pub(crate) fn new(new_aleph_network: DN, old_aleph_network: AlephNetwork<B>) -> Self {
        SplicedAlephNetwork {
            new_aleph_network,
            old_aleph_network,
        }
    }
}

#[async_trait::async_trait]
impl<B: Block, DN: DataNetwork<AlephNetworkData<B>>> DataNetwork<AlephNetworkData<B>>
    for SplicedAlephNetwork<B, DN>
{
    fn send(&self, data: AlephNetworkData<B>, recipient: Recipient) -> Result<(), SendError> {
        let _ = self.old_aleph_network.send(data.clone(), recipient.clone());
        self.new_aleph_network.send(data, recipient)
    }

    async fn next(&mut self) -> Option<AlephNetworkData<B>> {
        tokio::select! {
            data = self.old_aleph_network.next_event() => data,
            data = self.new_aleph_network.next() => data,
        }
    }
}

pub struct SplicedRmcNetwork<B: Block, DN: DataNetwork<RmcNetworkData<B>>> {
    new_rmc_network: DN,
    old_rmc_network: RmcNetwork<B>,
}

impl<B: Block, DN: DataNetwork<RmcNetworkData<B>>> SplicedRmcNetwork<B, DN> {
    pub(crate) fn new(new_rmc_network: DN, old_rmc_network: RmcNetwork<B>) -> Self {
        SplicedRmcNetwork {
            new_rmc_network,
            old_rmc_network,
        }
    }
}

#[async_trait::async_trait]
impl<B: Block, DN: DataNetwork<RmcNetworkData<B>>> DataNetwork<RmcNetworkData<B>>
    for SplicedRmcNetwork<B, DN>
{
    fn send(&self, data: RmcNetworkData<B>, recipient: Recipient) -> Result<(), SendError> {
        let _ = self
            .old_rmc_network
            .send(data.clone(), recipient.clone().into());
        self.new_rmc_network.send(data, recipient)
    }

    async fn next(&mut self) -> Option<RmcNetworkData<B>> {
        tokio::select! {
            data = self.old_rmc_network.next() => data,
            data = self.new_rmc_network.next() => data,
        }
    }
}
