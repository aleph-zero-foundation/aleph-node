use std::fmt::{Display, Error as FmtError, Formatter};

use aleph_primitives::AuthorityId;
use futures::channel::mpsc;

use crate::{
    crypto::AuthorityPen,
    validator_network::{
        io::{ReceiveError, SendError},
        Data, Splittable,
    },
};

mod handshake;
mod negotiation;
mod v0;

use handshake::HandshakeError;
pub use negotiation::{protocol, ProtocolNegotiationError};

pub type Version = u32;

/// The types of connections needed for backwards compatibility with the legacy two connections
/// protocol. Remove after it's no longer needed.
#[derive(PartialEq, Debug, Eq, Clone, Copy)]
pub enum ConnectionType {
    LegacyIncoming,
    LegacyOutgoing,
}

/// What connections send back to the service after they become established. Starts with a peer id
/// of the remote node, followed by a channel for sending data to that node, with None if the
/// connection was unsuccessful and should be reestablished. Finally a marker for legacy
/// compatibility.
pub type ResultForService<D> = (
    AuthorityId,
    Option<mpsc::UnboundedSender<D>>,
    ConnectionType,
);

/// Defines the protocol for communication.
#[derive(Debug, PartialEq, Eq)]
pub enum Protocol {
    /// The first version of the protocol, with unidirectional connections.
    V0,
}

/// Protocol error.
#[derive(Debug)]
pub enum ProtocolError {
    /// Error during performing a handshake.
    HandshakeError(HandshakeError),
    /// Sending failed.
    SendError(SendError),
    /// Receiving failed.
    ReceiveError(ReceiveError),
    /// Heartbeat stopped.
    CardiacArrest,
    /// Channel to the parent service closed.
    NoParentConnection,
    /// Data channel closed.
    NoUserConnection,
}

impl Display for ProtocolError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use ProtocolError::*;
        match self {
            HandshakeError(e) => write!(f, "handshake error: {}", e),
            SendError(e) => write!(f, "send error: {}", e),
            ReceiveError(e) => write!(f, "receive error: {}", e),
            CardiacArrest => write!(f, "heartbeat stopped"),
            NoParentConnection => write!(f, "cannot send result to service"),
            NoUserConnection => write!(f, "cannot send data to user"),
        }
    }
}

impl From<HandshakeError> for ProtocolError {
    fn from(e: HandshakeError) -> Self {
        ProtocolError::HandshakeError(e)
    }
}

impl From<SendError> for ProtocolError {
    fn from(e: SendError) -> Self {
        ProtocolError::SendError(e)
    }
}

impl From<ReceiveError> for ProtocolError {
    fn from(e: ReceiveError) -> Self {
        ProtocolError::ReceiveError(e)
    }
}

impl Protocol {
    /// Minimal supported protocol version.
    const MIN_VERSION: Version = 0;

    /// Maximal supported protocol version.
    const MAX_VERSION: Version = 0;

    /// Launches the proper variant of the protocol (receiver half).
    pub async fn manage_incoming<D: Data, S: Splittable>(
        &self,
        stream: S,
        authority_pen: AuthorityPen,
        result_for_service: mpsc::UnboundedSender<ResultForService<D>>,
        data_for_user: mpsc::UnboundedSender<D>,
    ) -> Result<(), ProtocolError> {
        use Protocol::*;
        match self {
            V0 => v0::incoming(stream, authority_pen, result_for_service, data_for_user).await,
        }
    }

    /// Launches the proper variant of the protocol (sender half).
    pub async fn manage_outgoing<D: Data, S: Splittable>(
        &self,
        stream: S,
        authority_pen: AuthorityPen,
        peer_id: AuthorityId,
        result_for_service: mpsc::UnboundedSender<ResultForService<D>>,
        _data_for_user: mpsc::UnboundedSender<D>,
    ) -> Result<(), ProtocolError> {
        use Protocol::*;
        match self {
            V0 => v0::outgoing(stream, authority_pen, peer_id, result_for_service).await,
        }
    }
}

impl TryFrom<Version> for Protocol {
    type Error = Version;

    fn try_from(version: Version) -> Result<Self, Self::Error> {
        match version {
            0 => Ok(Protocol::V0),
            unknown_version => Err(unknown_version),
        }
    }
}
