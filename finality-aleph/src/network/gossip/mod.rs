//! A P2P-based gossip network, for now only for sending broadcasts.
use std::{
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
/// Interface for the gossip network, currently only supports broadcasting and receiving data.
pub trait Network<D: Data>: Send + 'static {
    type Error: Display + Send;

    /// Broadcast data to all directly connected peers. Network-wide broadcasts have to be
    /// implemented on top of this abstraction. Note that there might be no currently connected
    /// peers, so there are no guarantees any single call sends anything even if no errors are
    /// returned, retry appropriately.
    fn broadcast(&mut self, data: D) -> Result<(), Self::Error>;

    /// Receive some data from the network.
    async fn next(&mut self) -> Result<D, Self::Error>;
}

/// The Authentication protocol is used for validator discovery.
#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub enum Protocol {
    Authentication,
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
    Messages(Vec<(Protocol, Bytes)>),
}

#[async_trait::async_trait]
pub trait EventStream<P> {
    async fn next_event(&mut self) -> Option<Event<P>>;
}

/// Abstraction over a raw p2p network.
pub trait RawNetwork: Clone + Send + Sync + 'static {
    type SenderError: std::error::Error;
    type NetworkSender: NetworkSender;
    type PeerId: Clone + Debug + Eq + Hash + Send;
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
