use std::{io::Result as IoResult, net::ToSocketAddrs as _};

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
    network::{Multiaddress, NetworkIdentity, PeerId},
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
pub struct TcpMultiaddress {
    peer_id: AuthorityId,
    address: String,
}

impl Multiaddress for TcpMultiaddress {
    type PeerId = AuthorityId;

    fn get_peer_id(&self) -> Option<Self::PeerId> {
        Some(self.peer_id.clone())
    }

    fn add_matching_peer_id(self, peer_id: Self::PeerId) -> Option<Self> {
        match self.peer_id == peer_id {
            true => Some(self),
            false => None,
        }
    }
}

#[derive(Clone)]
struct TcpDialer;

#[async_trait::async_trait]
impl Dialer<TcpMultiaddress> for TcpDialer {
    type Connection = TcpStream;
    type Error = std::io::Error;

    async fn connect(
        &mut self,
        addresses: Vec<TcpMultiaddress>,
    ) -> Result<Self::Connection, Self::Error> {
        let parsed_addresses: Vec<_> = addresses
            .into_iter()
            .filter_map(|address| address.address.to_socket_addrs().ok())
            .flatten()
            .collect();
        let stream = TcpStream::connect(&parsed_addresses[..]).await?;
        if stream.set_linger(None).is_err() {
            info!(target: "validator-network", "stream.set_linger(None) failed.");
        };
        Ok(stream)
    }
}

struct TcpNetworkIdentity {
    peer_id: AuthorityId,
    addresses: Vec<TcpMultiaddress>,
}

impl NetworkIdentity for TcpNetworkIdentity {
    type PeerId = AuthorityId;
    type Multiaddress = TcpMultiaddress;

    fn identity(&self) -> (Vec<Self::Multiaddress>, Self::PeerId) {
        (self.addresses.clone(), self.peer_id.clone())
    }
}

impl TcpNetworkIdentity {
    fn new(external_addresses: Vec<String>, peer_id: AuthorityId) -> TcpNetworkIdentity {
        TcpNetworkIdentity {
            addresses: external_addresses
                .into_iter()
                .map(|address| TcpMultiaddress {
                    peer_id: peer_id.clone(),
                    address,
                })
                .collect(),
            peer_id,
        }
    }
}

/// Create a new tcp network, including an identity that can be used for constructing
/// authentications for other peers.
pub async fn new_tcp_network<A: ToSocketAddrs>(
    listening_addresses: A,
    external_addresses: Vec<String>,
    peer_id: AuthorityId,
) -> IoResult<(
    impl Dialer<TcpMultiaddress>,
    impl Listener,
    impl NetworkIdentity<Multiaddress = TcpMultiaddress, PeerId = AuthorityId>,
)> {
    let listener = TcpListener::bind(listening_addresses).await?;
    let identity = TcpNetworkIdentity::new(external_addresses, peer_id);
    Ok((TcpDialer {}, listener, identity))
}

#[cfg(test)]
pub mod testing {
    use aleph_primitives::AuthorityId;

    use super::{TcpMultiaddress, TcpNetworkIdentity};
    use crate::network::NetworkIdentity;

    pub fn new_identity(
        external_addresses: Vec<String>,
        peer_id: AuthorityId,
    ) -> impl NetworkIdentity<Multiaddress = TcpMultiaddress, PeerId = AuthorityId> {
        TcpNetworkIdentity::new(external_addresses, peer_id)
    }
}
