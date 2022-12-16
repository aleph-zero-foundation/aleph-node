use std::fmt::{Display, Error as FmtError, Formatter};

use futures::channel::mpsc;

use crate::network::clique::{
    io::{ReceiveError, SendError},
    Data, PublicKey, SecretKey, Splittable,
};

mod handshake;
mod negotiation;
mod v0;
mod v1;

use handshake::HandshakeError;
pub use negotiation::{protocol, ProtocolNegotiationError};

pub type Version = u32;

/// The types of connections needed for backwards compatibility with the legacy two connections
/// protocol. Remove after it's no longer needed.
#[derive(PartialEq, Debug, Eq, Clone, Copy)]
pub enum ConnectionType {
    New,
    LegacyIncoming,
    LegacyOutgoing,
}

/// What connections send back to the service after they become established. Starts with a public
/// key of the remote node, followed by a channel for sending data to that node, with None if the
/// connection was unsuccessful and should be reestablished. Finally a marker for legacy
/// compatibility.
pub type ResultForService<PK, D> = (PK, Option<mpsc::UnboundedSender<D>>, ConnectionType);

/// Defines the protocol for communication.
#[derive(Debug, PartialEq, Eq)]
pub enum Protocol {
    /// The first version of the protocol, with unidirectional connections.
    V0,
    /// The current version of the protocol, with pseudorandom connection direction and
    /// multiplexing.
    V1,
}

/// Protocol error.
#[derive(Debug)]
pub enum ProtocolError<PK: PublicKey> {
    /// Error during performing a handshake.
    HandshakeError(HandshakeError<PK>),
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

impl<PK: PublicKey> Display for ProtocolError<PK> {
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

impl<PK: PublicKey> From<HandshakeError<PK>> for ProtocolError<PK> {
    fn from(e: HandshakeError<PK>) -> Self {
        ProtocolError::HandshakeError(e)
    }
}

impl<PK: PublicKey> From<SendError> for ProtocolError<PK> {
    fn from(e: SendError) -> Self {
        ProtocolError::SendError(e)
    }
}

impl<PK: PublicKey> From<ReceiveError> for ProtocolError<PK> {
    fn from(e: ReceiveError) -> Self {
        ProtocolError::ReceiveError(e)
    }
}

impl Protocol {
    /// Minimal supported protocol version.
    const MIN_VERSION: Version = 0;

    /// Maximal supported protocol version.
    const MAX_VERSION: Version = 1;

    /// Launches the proper variant of the protocol (receiver half).
    pub async fn manage_incoming<SK: SecretKey, D: Data, S: Splittable>(
        &self,
        stream: S,
        secret_key: SK,
        result_for_service: mpsc::UnboundedSender<ResultForService<SK::PublicKey, D>>,
        data_for_user: mpsc::UnboundedSender<D>,
    ) -> Result<(), ProtocolError<SK::PublicKey>> {
        use Protocol::*;
        match self {
            V0 => v0::incoming(stream, secret_key, result_for_service, data_for_user).await,
            V1 => v1::incoming(stream, secret_key, result_for_service, data_for_user).await,
        }
    }

    /// Launches the proper variant of the protocol (sender half).
    pub async fn manage_outgoing<SK: SecretKey, D: Data, S: Splittable>(
        &self,
        stream: S,
        secret_key: SK,
        public_key: SK::PublicKey,
        result_for_service: mpsc::UnboundedSender<ResultForService<SK::PublicKey, D>>,
        data_for_user: mpsc::UnboundedSender<D>,
    ) -> Result<(), ProtocolError<SK::PublicKey>> {
        use Protocol::*;
        match self {
            V0 => v0::outgoing(stream, secret_key, public_key, result_for_service).await,
            V1 => {
                v1::outgoing(
                    stream,
                    secret_key,
                    public_key,
                    result_for_service,
                    data_for_user,
                )
                .await
            }
        }
    }
}

impl TryFrom<Version> for Protocol {
    type Error = Version;

    fn try_from(version: Version) -> Result<Self, Self::Error> {
        match version {
            0 => Ok(Protocol::V0),
            1 => Ok(Protocol::V1),
            unknown_version => Err(unknown_version),
        }
    }
}
