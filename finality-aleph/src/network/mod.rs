use codec::Codec;

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

use crate::BlockIdentifier;

/// Abstraction for requesting justifications for finalized blocks and stale blocks.
pub trait RequestBlocks<BI: BlockIdentifier>: Clone + Send + Sync + 'static {
    /// Request the justification for the given block
    fn request_justification(&self, block: BI);

    /// Request the given block -- this is supposed to be used only for "old forks".
    fn request_stale_block(&self, block: BI);

    /// Clear all pending justification requests. We need this function in case
    /// we requested a justification for a block, which will never get it.
    fn clear_justification_requests(&self);
}

/// A basic alias for properties we expect basic data to satisfy.
pub trait Data: Clone + Codec + Send + Sync + 'static {}

impl<D: Clone + Codec + Send + Sync + 'static> Data for D {}
