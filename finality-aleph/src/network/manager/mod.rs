use codec::{Decode, Encode, Error, Input, Output};

use crate::{
    crypto::Signature,
    network::{AddressingInformation, Data},
    NodeIndex, SessionId,
};

mod compatibility;
mod connections;
mod discovery;
mod service;
mod session;

pub use compatibility::{
    DiscoveryMessage, LegacyDiscoveryMessage, PeerAuthentications, VersionedAuthentication,
};
use connections::Connections;
pub use discovery::Discovery;
pub use service::{
    Config as ConnectionManagerConfig, Service as ConnectionManager, SessionCommand,
    IO as ConnectionIO,
};
pub use session::{Handler as SessionHandler, HandlerError as SessionHandlerError};

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

/// Data inside session, sent to validator network.
/// Wrapper for data send over network. We need it to ensure compatibility.
/// The order of the data and session_id is fixed in encode and the decode expects it to be data, session_id.
/// Since data is versioned, i.e. it's encoding starts with a version number in the standardized way,
/// this will allow us to retrofit versioning here if we ever need to change this structure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataInSession<D: Data> {
    pub data: D,
    pub session_id: SessionId,
}

impl<D: Data> Decode for DataInSession<D> {
    fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
        let data = D::decode(input)?;
        let session_id = SessionId::decode(input)?;

        Ok(Self { data, session_id })
    }
}

impl<D: Data> Encode for DataInSession<D> {
    fn size_hint(&self) -> usize {
        self.data.size_hint() + self.session_id.size_hint()
    }

    fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
        self.data.encode_to(dest);
        self.session_id.encode_to(dest);
    }
}
