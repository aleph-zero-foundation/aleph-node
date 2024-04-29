pub mod address_cache;
pub mod data;
#[cfg(test)]
pub mod mock;
pub mod session;
mod substrate;
pub mod tcp;

use std::{
    collections::HashSet,
    fmt::{Debug, Display},
    hash::Hash,
};

use network_clique::{AddressingInformation, NetworkIdentity, PeerId};
use parity_scale_codec::Codec;
use sc_network::config::FullNetworkConfiguration;
use sc_service::Configuration;
use session::MAX_MESSAGE_SIZE as MAX_AUTHENTICATION_MESSAGE_SIZE;
pub use substrate::{PeerId as SubstratePeerId, ProtocolNetwork, SyncNetworkService};

use crate::sync::MAX_MESSAGE_SIZE as MAX_BLOCK_SYNC_MESSAGE_SIZE;

/// A basic alias for properties we expect basic data to satisfy.
pub trait Data: Clone + Codec + Send + Sync + 'static {}

impl<D: Clone + Codec + Send + Sync + 'static> Data for D {}

/// Name of the network protocol used by Aleph Zero to disseminate validator
/// authentications.
const AUTHENTICATION_PROTOCOL_NAME: &str = "/auth/0";

/// Name of the network protocol used by Aleph Zero to synchronize the block state.
const BLOCK_SYNC_PROTOCOL_NAME: &str = "/sync/0";

/// Struct containing network configuration and networks for every protocol we use.
pub struct NetConfig {
    /// Full network configuration.
    pub net_config: FullNetworkConfiguration,
    /// Authentication network.
    pub authentication_network: ProtocolNetwork,
    /// Block sync network.
    pub block_sync_network: ProtocolNetwork,
}

impl NetConfig {
    fn add_protocol(
        chain_prefix: &str,
        protocol_name: &str,
        max_message_size: u64,
        net_config: &mut FullNetworkConfiguration,
    ) -> ProtocolNetwork {
        let (config, notifications) = sc_network::config::NonDefaultSetConfig::new(
            // full protocol name
            format!("{chain_prefix}{protocol_name}").into(),
            // no fallback names
            vec![],
            max_message_size,
            // we do not use custom handshake
            None,
            sc_network::config::SetConfig::default(),
        );
        net_config.add_notification_protocol(config);
        ProtocolNetwork::new(notifications)
    }

    /// Create the full configuration and networks per protocol.
    pub fn new(config: &Configuration, chain_prefix: &str) -> Self {
        let mut net_config = FullNetworkConfiguration::new(&config.network);

        let authentication_network = Self::add_protocol(
            chain_prefix,
            AUTHENTICATION_PROTOCOL_NAME,
            MAX_AUTHENTICATION_MESSAGE_SIZE,
            &mut net_config,
        );
        let block_sync_network = Self::add_protocol(
            chain_prefix,
            BLOCK_SYNC_PROTOCOL_NAME,
            MAX_BLOCK_SYNC_MESSAGE_SIZE,
            &mut net_config,
        );

        Self {
            net_config,
            authentication_network,
            block_sync_network,
        }
    }
}

#[async_trait::async_trait]
/// Interface for the gossip network. This represents a P2P network and a lot of the properties of
/// this interface result from that. In particular we might know the ID of a given peer, but not be
/// connected to them directly.
pub trait GossipNetwork<D: Data>: Send + 'static {
    type Error: Display + Send;
    type PeerId: Clone + Debug + Eq + Hash + Send + 'static;

    /// Attempt to send data to a peer. Might silently fail if we are not connected to them.
    fn send_to(&mut self, data: D, peer_id: Self::PeerId) -> Result<(), Self::Error>;

    /// Send data to a random peer, preferably from a list. It should send the data to a randomly
    /// chosen peer from the provided list, but if it cannot (e.g. because it's not connected) it
    /// will send to a random available peer. No guarantees any peer gets it even if no errors are
    /// returned, retry appropriately.
    fn send_to_random(
        &mut self,
        data: D,
        peer_ids: HashSet<Self::PeerId>,
    ) -> Result<(), Self::Error>;

    /// Broadcast data to all directly connected peers. Network-wide broadcasts have to be
    /// implemented on top of this abstraction. Note that there might be no currently connected
    /// peers, so there are no guarantees any single call sends anything even if no errors are
    /// returned, retry appropriately.
    fn broadcast(&mut self, data: D) -> Result<(), Self::Error>;

    /// Receive some data from the network, including information about who sent it.
    /// This method's implementation must be cancellation safe.
    async fn next(&mut self) -> Result<(D, Self::PeerId), Self::Error>;
}
