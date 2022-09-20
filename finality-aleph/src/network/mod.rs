use std::{
    collections::HashSet,
    fmt::{Debug, Display},
    hash::Hash,
};

use aleph_bft::Recipient;
use async_trait::async_trait;
use bytes::Bytes;
use codec::Codec;
use sp_api::NumberFor;
use sp_runtime::traits::Block;

mod aleph;
mod component;
mod manager;
#[cfg(test)]
pub mod mock;
mod service;
mod session;
mod split;

pub use aleph::{NetworkData as AlephNetworkData, NetworkWrapper};
pub use component::{
    Network as ComponentNetwork, NetworkExt as ComponentNetworkExt,
    NetworkMap as ComponentNetworkMap, Receiver as ReceiverComponent, Sender as SenderComponent,
    SimpleNetwork,
};
use manager::SessionCommand;
pub use manager::{ConnectionIO, ConnectionManager, ConnectionManagerConfig};
pub use service::{Service, IO};
pub use session::{Manager as SessionManager, ManagerError};
pub use split::{split, Split};

#[cfg(test)]
pub mod testing {
    pub use super::manager::{Authentication, DiscoveryMessage, NetworkData, SessionHandler};
}

/// Represents the id of an arbitrary node.
pub trait PeerId: PartialEq + Eq + Copy + Clone + Debug + Display + Hash + Codec + Send {}

/// Represents the address of an arbitrary node.
pub trait Multiaddress: Debug + Hash + Codec + Clone + Eq {
    type PeerId: PeerId;

    /// Returns the peer id associated with this multiaddress if it exists and is unique.
    fn get_peer_id(&self) -> Option<Self::PeerId>;

    /// Returns the address extended by the peer id, unless it already contained another peer id.
    fn add_matching_peer_id(self, peer_id: Self::PeerId) -> Option<Self>;
}

/// The Generic protocol is used for validator discovery.
/// The Validator protocol is used for validator-specific messages, i.e. ones needed for
/// finalization.
#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub enum Protocol {
    Generic,
    Validator,
}

/// Abstraction over a sender to network.
#[async_trait]
pub trait NetworkSender: Send + Sync + 'static {
    type SenderError: std::error::Error;

    /// A method for sending data. Returns Error if not connected to the peer.
    async fn send<'a>(
        &'a self,
        data: impl Into<Vec<u8>> + Send + Sync + 'static,
    ) -> Result<(), Self::SenderError>;
}

#[derive(Clone)]
pub enum Event<M: Multiaddress> {
    Connected(M),
    Disconnected(M::PeerId),
    StreamOpened(M::PeerId, Protocol),
    StreamClosed(M::PeerId, Protocol),
    Messages(Vec<Bytes>),
}

#[async_trait]
pub trait EventStream<M: Multiaddress> {
    async fn next_event(&mut self) -> Option<Event<M>>;
}

/// Abstraction over a network.
pub trait Network: Clone + Send + Sync + 'static {
    type SenderError: std::error::Error;
    type NetworkSender: NetworkSender;
    type PeerId: PeerId;
    type Multiaddress: Multiaddress<PeerId = Self::PeerId>;
    type EventStream: EventStream<Self::Multiaddress>;

    /// Returns a stream of events representing what happens on the network.
    fn event_stream(&self) -> Self::EventStream;

    /// Returns a sender to the given peer using a given protocol. Returns Error if not connected to the peer.
    fn sender(
        &self,
        peer_id: Self::PeerId,
        protocol: Protocol,
    ) -> Result<Self::NetworkSender, Self::SenderError>;

    /// Add peers to one of the reserved sets.
    fn add_reserved(&self, addresses: HashSet<Self::Multiaddress>, protocol: Protocol);

    /// Remove peers from one of the reserved sets.
    fn remove_reserved(&self, peers: HashSet<Self::PeerId>, protocol: Protocol);
}

/// Abstraction for requesting own network addresses and PeerId.
pub trait NetworkIdentity {
    type PeerId: PeerId;
    type Multiaddress: Multiaddress<PeerId = Self::PeerId>;

    /// The external identity of this node, consisting of addresses and the PeerId.
    fn identity(&self) -> (Vec<Self::Multiaddress>, Self::PeerId);
}

/// Abstraction for requesting justifications for finalized blocks and stale blocks.
pub trait RequestBlocks<B: Block>: Clone + Send + Sync + 'static {
    /// Request the justification for the given block
    fn request_justification(&self, hash: &B::Hash, number: NumberFor<B>);

    /// Request the given block -- this is supposed to be used only for "old forks".
    fn request_stale_block(&self, hash: B::Hash, number: NumberFor<B>);

    /// Clear all pending justification requests. We need this function in case
    /// we requested a justification for a block, which will never get it.
    fn clear_justification_requests(&self);

    /// Are we in the process of downloading the chain?
    ///
    /// Like [`NetworkService::is_major_syncing`][1].
    ///
    /// [1]: sc_network::NetworkService::is_major_syncing
    fn is_major_syncing(&self) -> bool;
}

/// What do do with a specific piece of data.
/// Note that broadcast does not specify the protocol, as we only broadcast Generic messages in this sense.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DataCommand<PID: PeerId> {
    Broadcast,
    SendTo(PID, Protocol),
}

/// Commands for manipulating the reserved peers set.
#[derive(Debug, PartialEq, Eq)]
pub enum ConnectionCommand<M: Multiaddress> {
    AddReserved(HashSet<M>),
    DelReserved(HashSet<M::PeerId>),
}

/// Returned when something went wrong when sending data using a DataNetwork.
#[derive(Debug)]
pub enum SendError {
    SendFailed,
}

/// What the data sent using the network has to satisfy.
pub trait Data: Clone + Codec + Send + Sync + 'static {}

impl<D: Clone + Codec + Send + Sync + 'static> Data for D {}

/// A generic interface for sending and receiving data.
#[async_trait::async_trait]
pub trait DataNetwork<D: Data>: Send + Sync {
    fn send(&self, data: D, recipient: Recipient) -> Result<(), SendError>;
    async fn next(&mut self) -> Option<D>;
}
