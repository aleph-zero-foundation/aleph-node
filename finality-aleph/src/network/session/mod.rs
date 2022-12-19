//! Managing the validator connections in sessions using the gossip network.
use std::fmt::Display;

use codec::{Decode, Encode};
use futures::channel::mpsc;

use crate::{
    crypto::{AuthorityPen, AuthorityVerifier, Signature},
    network::{
        data::{
            component::{Sender, SimpleNetwork},
            SendError,
        },
        AddressingInformation, Data,
    },
    NodeIndex, Recipient, SessionId,
};

mod compatibility;
mod connections;
mod data;
mod discovery;
mod handler;
mod manager;
mod service;

pub use compatibility::{
    DiscoveryMessage, LegacyDiscoveryMessage, PeerAuthentications, VersionedAuthentication,
};
use connections::Connections;
#[cfg(test)]
pub use data::DataInSession;
pub use discovery::Discovery;
#[cfg(test)]
pub use handler::tests::{authentication, legacy_authentication};
pub use handler::{Handler as SessionHandler, HandlerError as SessionHandlerError};
pub use service::{Config as ConnectionManagerConfig, ManagerError, Service as ConnectionManager};

/// Data validators used to use to authenticate themselves for a single session
/// and disseminate their addresses.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Encode, Decode)]
pub struct LegacyAuthData<M: Data> {
    addresses: Vec<M>,
    node_id: NodeIndex,
    session_id: SessionId,
}

impl<M: Data> LegacyAuthData<M> {
    pub fn session(&self) -> SessionId {
        self.session_id
    }

    pub fn creator(&self) -> NodeIndex {
        self.node_id
    }
}

/// Data validators use to authenticate themselves for a single session
/// and disseminate their addresses.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Encode, Decode)]
pub struct AuthData<A: AddressingInformation> {
    address: A,
    node_id: NodeIndex,
    session_id: SessionId,
}

impl<A: AddressingInformation> AuthData<A> {
    pub fn session(&self) -> SessionId {
        self.session_id
    }

    pub fn creator(&self) -> NodeIndex {
        self.node_id
    }

    pub fn address(&self) -> A {
        self.address.clone()
    }
}

impl<M: Data, A: AddressingInformation + Into<Vec<M>>> From<AuthData<A>> for LegacyAuthData<M> {
    fn from(auth_data: AuthData<A>) -> Self {
        let AuthData {
            address,
            node_id,
            session_id,
        } = auth_data;
        let addresses = address.into();
        LegacyAuthData {
            addresses,
            node_id,
            session_id,
        }
    }
}

impl<M: Data, A: AddressingInformation + TryFrom<Vec<M>>> TryFrom<LegacyAuthData<M>>
    for AuthData<A>
{
    type Error = ();

    fn try_from(legacy_auth_data: LegacyAuthData<M>) -> Result<Self, Self::Error> {
        let LegacyAuthData {
            addresses,
            node_id,
            session_id,
        } = legacy_auth_data;
        let address = addresses.try_into().map_err(|_| ())?;
        Ok(AuthData {
            address,
            node_id,
            session_id,
        })
    }
}

/// A full legacy authentication, consisting of a signed LegacyAuthData.
pub type LegacyAuthentication<M> = (LegacyAuthData<M>, Signature);

/// A full authentication, consisting of a signed AuthData.
pub type Authentication<A> = (AuthData<A>, Signature);

/// Sends data within a single session.
#[derive(Clone)]
pub struct SessionSender<D: Data> {
    session_id: SessionId,
    messages_for_network: mpsc::UnboundedSender<(D, SessionId, Recipient)>,
}

impl<D: Data> Sender<D> for SessionSender<D> {
    fn send(&self, data: D, recipient: Recipient) -> Result<(), SendError> {
        self.messages_for_network
            .unbounded_send((data, self.session_id, recipient))
            .map_err(|_| SendError::SendFailed)
    }
}

/// Sends and receives data within a single session.
type Network<D> = SimpleNetwork<D, mpsc::UnboundedReceiver<D>, SessionSender<D>>;

/// An interface for managing session networks for validators and nonvalidators.
#[async_trait::async_trait]
pub trait SessionManager<D: Data>: Send + Sync + 'static {
    type Error: Display;

    /// Start participating or update the verifier in the given session where you are not a
    /// validator.
    fn start_nonvalidator_session(
        &self,
        session_id: SessionId,
        verifier: AuthorityVerifier,
    ) -> Result<(), Self::Error>;

    /// Start participating or update the information about the given session where you are a
    /// validator. Returns a session network to be used for sending and receiving data within the
    /// session.
    async fn start_validator_session(
        &self,
        session_id: SessionId,
        verifier: AuthorityVerifier,
        node_id: NodeIndex,
        pen: AuthorityPen,
    ) -> Result<Network<D>, Self::Error>;

    /// Start participating or update the information about the given session where you are a
    /// validator. Used for early starts when you don't yet need the returned network, but would
    /// like to start discovery.
    fn early_start_validator_session(
        &self,
        session_id: SessionId,
        verifier: AuthorityVerifier,
        node_id: NodeIndex,
        pen: AuthorityPen,
    ) -> Result<(), Self::Error>;

    /// Stop participating in the given session.
    fn stop_session(&self, session_id: SessionId) -> Result<(), Self::Error>;
}
