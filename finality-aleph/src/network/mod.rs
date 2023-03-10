use codec::Codec;
use sp_api::NumberFor;
use sp_runtime::traits::Block;

pub mod data;
mod gossip;
#[cfg(test)]
pub mod mock;
pub mod session;
mod substrate;
pub mod tcp;

#[cfg(test)]
pub use gossip::mock::{MockEvent, MockRawNetwork};
pub use gossip::{Network as GossipNetwork, Protocol, Service as GossipService};
use network_clique::{AddressingInformation, NetworkIdentity, PeerId};
pub use substrate::{ProtocolNaming, SubstrateNetwork};

/// Abstraction for requesting justifications for finalized blocks and stale blocks.
pub trait RequestBlocks<B: Block>: Clone + Send + Sync + 'static {
    /// Request the justification for the given block
    fn request_justification(&self, hash: &B::Hash, number: NumberFor<B>);

    /// Request the given block -- this is supposed to be used only for "old forks".
    fn request_stale_block(&self, hash: B::Hash, number: NumberFor<B>);

    /// Clear all pending justification requests. We need this function in case
    /// we requested a justification for a block, which will never get it.
    fn clear_justification_requests(&self);
}

/// A basic alias for properties we expect basic data to satisfy.
pub trait Data: Clone + Codec + Send + Sync + 'static {}

impl<D: Clone + Codec + Send + Sync + 'static> Data for D {}
