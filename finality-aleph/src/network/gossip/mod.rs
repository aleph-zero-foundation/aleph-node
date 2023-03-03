//! A P2P-based gossip network, for now only for sending broadcasts.
use std::{
    collections::HashSet,
    fmt::{Debug, Display},
    hash::Hash,
};

use bytes::Bytes;

use crate::network::Data;

#[cfg(test)]
pub mod mock;
mod service;

pub use service::Service;

#[async_trait::async_trait]
/// Interface for the gossip network. This represents a P2P network and a lot of the properties of
/// this interface result from that. In particular we might know the ID of a given peer, but not be
/// connected to them directly.
pub trait Network<D: Data>: Send + 'static {
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
    async fn next(&mut self) -> Result<(D, Self::PeerId), Self::Error>;
}

/// Protocols used by the network.
#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub enum Protocol {
    /// The authentication protocol is used for validator discovery.
    Authentication,
    /// The block synchronization protocol.
    BlockSync,
}

/// Abstraction over a sender to the raw network.
#[async_trait::async_trait]
pub trait NetworkSender: Send + Sync + 'static {
    type SenderError: std::error::Error;

    /// A method for sending data. Returns Error if not connected to the peer.
    async fn send<'a>(
        &'a self,
        data: impl Into<Vec<u8>> + Send + Sync + 'static,
    ) -> Result<(), Self::SenderError>;
}

#[derive(Clone)]
pub enum Event<P> {
    StreamOpened(P, Protocol),
    StreamClosed(P, Protocol),
    Messages(P, Vec<(Protocol, Bytes)>),
}

#[async_trait::async_trait]
pub trait EventStream<P> {
    async fn next_event(&mut self) -> Option<Event<P>>;
}

/// Abstraction over a raw p2p network.
pub trait RawNetwork: Clone + Send + Sync + 'static {
    type SenderError: std::error::Error;
    type NetworkSender: NetworkSender;
    type PeerId: Clone + Debug + Eq + Hash + Send + 'static;
    type EventStream: EventStream<Self::PeerId>;

    /// Returns a stream of events representing what happens on the network.
    fn event_stream(&self) -> Self::EventStream;

    /// Returns a sender to the given peer using a given protocol. Returns Error if not connected to the peer.
    fn sender(
        &self,
        peer_id: Self::PeerId,
        protocol: Protocol,
    ) -> Result<Self::NetworkSender, Self::SenderError>;
}
