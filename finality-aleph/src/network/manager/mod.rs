use codec::{Decode, Encode, Error, Input, Output};

use crate::{
    crypto::Signature,
    network::{Data, Multiaddress},
    NodeIndex, SessionId,
};

mod compatibility;
mod connections;
mod discovery;
mod service;
mod session;

pub use compatibility::VersionedAuthentication;
use connections::Connections;
pub use discovery::{Discovery, DiscoveryMessage};
pub use service::{
    Config as ConnectionManagerConfig, Service as ConnectionManager, SessionCommand,
    IO as ConnectionIO,
};
pub use session::{Handler as SessionHandler, HandlerError as SessionHandlerError};

/// Data validators use to authenticate themselves for a single session
/// and disseminate their addresses.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Encode, Decode)]
pub struct AuthData<M: Multiaddress> {
    addresses: Vec<M>,
    node_id: NodeIndex,
    session_id: SessionId,
}

impl<M: Multiaddress> AuthData<M> {
    pub fn session(&self) -> SessionId {
        self.session_id
    }

    pub fn creator(&self) -> NodeIndex {
        self.node_id
    }

    pub fn addresses(&self) -> Vec<M> {
        self.addresses.clone()
    }
}

/// A full authentication, consisting of a signed AuthData.
pub type Authentication<M> = (AuthData<M>, Signature);

/// Data inside session, sent to validator network.
/// Wrapper for data send over network. We need it to ensure compatibility.
/// The order of the data and session_id is fixed in encode and the decode expects it to be data, session_id.
/// Since data is versioned, i.e. it's encoding starts with a version number in the standardized way,
/// this will allow us to retrofit versioning here if we ever need to change this structure.
#[derive(Clone)]
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

impl<D: Data, M: Multiaddress> From<DataInSession<D>> for NetworkData<D, M> {
    fn from(data: DataInSession<D>) -> Self {
        NetworkData::Data(data.data, data.session_id)
    }
}

/// The data that should be sent to the network service.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub enum NetworkData<D: Data, M: Multiaddress> {
    Meta(DiscoveryMessage<M>),
    Data(D, SessionId),
}
