use std::{io::Error as IoError, iter, net::ToSocketAddrs as _};

use aleph_primitives::AuthorityId;
use codec::{Decode, Encode};
use log::info;
use sp_core::crypto::KeyTypeId;
use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpListener, TcpStream, ToSocketAddrs,
};

use crate::{
    crypto::{verify, AuthorityPen, Signature},
    network::{AddressingInformation, NetworkIdentity, PeerId},
    validator_network::{ConnectionInfo, Dialer, Listener, PublicKey, SecretKey, Splittable},
};

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"a0vn");

impl ConnectionInfo for TcpStream {
    fn peer_address_info(&self) -> String {
        match self.peer_addr() {
            Ok(addr) => addr.to_string(),
            Err(e) => format!("unknown address: {}", e),
        }
    }
}

impl ConnectionInfo for OwnedWriteHalf {
    fn peer_address_info(&self) -> String {
        match self.peer_addr() {
            Ok(addr) => addr.to_string(),
            Err(e) => e.to_string(),
        }
    }
}

impl ConnectionInfo for OwnedReadHalf {
    fn peer_address_info(&self) -> String {
        match self.peer_addr() {
            Ok(addr) => addr.to_string(),
            Err(e) => e.to_string(),
        }
    }
}

impl Splittable for TcpStream {
    type Sender = OwnedWriteHalf;
    type Receiver = OwnedReadHalf;

    fn split(self) -> (Self::Sender, Self::Receiver) {
        let (receiver, sender) = self.into_split();
        (sender, receiver)
    }
}

#[async_trait::async_trait]
impl Listener for TcpListener {
    type Connection = TcpStream;
    type Error = std::io::Error;

    async fn accept(&mut self) -> Result<Self::Connection, Self::Error> {
        let stream = TcpListener::accept(self).await.map(|(stream, _)| stream)?;
        if stream.set_linger(None).is_err() {
            info!(target: "validator-network", "stream.set_linger(None) failed.");
        };
        Ok(stream)
    }
}

impl PeerId for AuthorityId {}

impl PublicKey for AuthorityId {
    type Signature = Signature;

    fn verify(&self, message: &[u8], signature: &Self::Signature) -> bool {
        verify(self, message, signature)
    }
}

#[async_trait::async_trait]
impl SecretKey for AuthorityPen {
    type Signature = Signature;
    type PublicKey = AuthorityId;

    async fn sign(&self, message: &[u8]) -> Self::Signature {
        AuthorityPen::sign(self, message).await
    }

    fn public_key(&self) -> Self::PublicKey {
        self.authority_id()
    }
}

/// A representation of a single TCP address with an associated peer ID.
#[derive(Debug, Hash, Encode, Decode, Clone, PartialEq, Eq)]
pub struct LegacyTcpMultiaddress {
    peer_id: AuthorityId,
    address: String,
}

/// What can go wrong when handling addressing information.
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub enum AddressingInformationError {
    /// Construction of an addressing information object requires at least one address.
    NoAddress,
}

/// A representation of TCP addressing information with an associated peer ID.
#[derive(Debug, Hash, Encode, Decode, Clone, PartialEq, Eq)]
pub struct TcpAddressingInformation {
    peer_id: AuthorityId,
    // Easiest way to ensure that the Vec below is nonempty...
    primary_address: String,
    other_addresses: Vec<String>,
}

impl TryFrom<Vec<LegacyTcpMultiaddress>> for TcpAddressingInformation {
    type Error = AddressingInformationError;

    fn try_from(legacy: Vec<LegacyTcpMultiaddress>) -> Result<Self, Self::Error> {
        let mut legacy = legacy.into_iter();
        let (peer_id, primary_address) = match legacy.next() {
            Some(LegacyTcpMultiaddress { peer_id, address }) => (peer_id, address),
            None => return Err(AddressingInformationError::NoAddress),
        };
        let other_addresses = legacy
            .filter(|la| la.peer_id == peer_id)
            .map(|la| la.address)
            .collect();
        Ok(TcpAddressingInformation {
            peer_id,
            primary_address,
            other_addresses,
        })
    }
}

impl From<TcpAddressingInformation> for Vec<LegacyTcpMultiaddress> {
    fn from(address: TcpAddressingInformation) -> Self {
        let TcpAddressingInformation {
            peer_id,
            primary_address,
            other_addresses,
        } = address;
        iter::once(primary_address)
            .chain(other_addresses)
            .map(|address| LegacyTcpMultiaddress {
                peer_id: peer_id.clone(),
                address,
            })
            .collect()
    }
}

impl AddressingInformation for TcpAddressingInformation {
    type PeerId = AuthorityId;

    fn peer_id(&self) -> Self::PeerId {
        self.peer_id.clone()
    }
}

impl NetworkIdentity for TcpAddressingInformation {
    type PeerId = AuthorityId;
    type AddressingInformation = TcpAddressingInformation;

    fn identity(&self) -> Self::AddressingInformation {
        self.clone()
    }
}

impl TcpAddressingInformation {
    fn new(
        addresses: Vec<String>,
        peer_id: AuthorityId,
    ) -> Result<TcpAddressingInformation, AddressingInformationError> {
        let mut addresses = addresses.into_iter();
        let primary_address = match addresses.next() {
            Some(address) => address,
            None => return Err(AddressingInformationError::NoAddress),
        };
        Ok(TcpAddressingInformation {
            primary_address,
            other_addresses: addresses.collect(),
            peer_id,
        })
    }
}

#[derive(Clone)]
struct TcpDialer;

#[async_trait::async_trait]
impl Dialer<TcpAddressingInformation> for TcpDialer {
    type Connection = TcpStream;
    type Error = std::io::Error;

    async fn connect(
        &mut self,
        address: TcpAddressingInformation,
    ) -> Result<Self::Connection, Self::Error> {
        let TcpAddressingInformation {
            primary_address,
            other_addresses,
            ..
        } = address;
        let parsed_addresses: Vec<_> = iter::once(primary_address)
            .chain(other_addresses)
            .filter_map(|address| address.to_socket_addrs().ok())
            .flatten()
            .collect();
        let stream = TcpStream::connect(&parsed_addresses[..]).await?;
        if stream.set_linger(None).is_err() {
            info!(target: "validator-network", "stream.set_linger(None) failed.");
        };
        Ok(stream)
    }
}

/// Possible errors when creating a TCP network.
#[derive(Debug)]
pub enum Error {
    Io(IoError),
    AddressingInformation(AddressingInformationError),
}

impl From<IoError> for Error {
    fn from(e: IoError) -> Self {
        Error::Io(e)
    }
}

impl From<AddressingInformationError> for Error {
    fn from(e: AddressingInformationError) -> Self {
        Error::AddressingInformation(e)
    }
}

/// Create a new tcp network, including an identity that can be used for constructing
/// authentications for other peers.
pub async fn new_tcp_network<A: ToSocketAddrs>(
    listening_addresses: A,
    external_addresses: Vec<String>,
    peer_id: AuthorityId,
) -> Result<
    (
        impl Dialer<TcpAddressingInformation>,
        impl Listener,
        impl NetworkIdentity<AddressingInformation = TcpAddressingInformation, PeerId = AuthorityId>,
    ),
    Error,
> {
    let listener = TcpListener::bind(listening_addresses).await?;
    let identity = TcpAddressingInformation::new(external_addresses, peer_id)?;
    Ok((TcpDialer {}, listener, identity))
}

#[cfg(test)]
pub mod testing {
    use aleph_primitives::AuthorityId;

    use super::TcpAddressingInformation;
    use crate::network::NetworkIdentity;

    /// Creates a realistic identity.
    pub fn new_identity(
        external_addresses: Vec<String>,
        peer_id: AuthorityId,
    ) -> impl NetworkIdentity<AddressingInformation = TcpAddressingInformation, PeerId = AuthorityId>
    {
        TcpAddressingInformation::new(external_addresses, peer_id)
            .expect("the provided addresses are fine")
    }
}
