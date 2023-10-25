use parity_scale_codec::Codec;

pub mod address_cache;
pub mod data;
mod gossip;
#[cfg(test)]
pub mod mock;
pub mod session;
mod substrate;
pub mod tcp;

#[cfg(test)]
pub use gossip::mock::{MockEvent, MockRawNetwork};
pub use gossip::{
    Error as GossipError, Network as GossipNetwork, Protocol, Service as GossipService,
};
use network_clique::{AddressingInformation, NetworkIdentity, PeerId};
pub use substrate::{ProtocolNaming, SubstrateNetwork};

/// A basic alias for properties we expect basic data to satisfy.
pub trait Data: Clone + Codec + Send + Sync + 'static {}

impl<D: Clone + Codec + Send + Sync + 'static> Data for D {}
