#![allow(dead_code)]
use std::fmt::Display;

use aleph_primitives::AuthorityId;
use codec::Codec;
use tokio::io::{AsyncRead, AsyncWrite};

mod handshake;
mod heartbeat;
mod io;
#[cfg(test)]
mod mock;
mod protocols;

/// What the data sent using the network has to satisfy.
pub trait Data: Clone + Codec + Send + Sync + 'static {}

impl<D: Clone + Codec + Send + Sync + 'static> Data for D {}

/// Network represents an interface for opening and closing connections with other Validators,
/// and sending direct messages between them.
///
/// Note on Network reliability and security: it is neither assumed that the sent messages must be
/// always delivered, nor the established connections must be secure in any way. The Network
/// implementation might fail to deliver any specific message, so messages have to be resent while
/// they still should be delivered.
#[async_trait::async_trait]
pub trait Network<A: Data, D: Data>: Send {
    /// Add the peer to the set of connected peers.
    fn add_connection(&mut self, peer: AuthorityId, addresses: Vec<A>);

    /// Remove the peer from the set of connected peers and close the connection.
    fn remove_connection(&mut self, peer: AuthorityId);

    /// Send a message to a single peer.
    /// This function should be implemented in a non-blocking manner.
    fn send(&self, data: D, recipient: AuthorityId);

    /// Receive a message from the network.
    async fn next(&mut self) -> Option<D>;
}

/// A stream that can be split into a sending and receiving part.
pub trait Splittable: AsyncWrite + AsyncRead + Unpin + Send {
    type Sender: AsyncWrite + Unpin + Send;
    type Receiver: AsyncRead + Unpin + Send;

    /// Split into the sending and receiving part.
    fn split(self) -> (Self::Sender, Self::Receiver);
}

/// Can use addresses to connect to a peer.
#[async_trait::async_trait]
pub trait Dialer<A: Data>: Clone + Send + 'static {
    type Connection: Splittable;
    type Error: Display;

    /// Attempt to connect to a peer using the provided addresses. Should work if at least one of
    /// the addresses is correct.
    async fn connect(&mut self, addresses: Vec<A>) -> Result<Self::Connection, Self::Error>;
}

/// Accepts new connections. Usually will be created listening on a specific interface and this is
/// just the result.
#[async_trait::async_trait]
pub trait Listener {
    type Connection: Splittable + 'static;
    type Error: Display;

    /// Returns the next incoming connection.
    async fn accept(&mut self) -> Result<Self::Connection, Self::Error>;
}
