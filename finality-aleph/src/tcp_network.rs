use std::{io::Result as IoResult, marker::PhantomData, net::ToSocketAddrs as _, sync::Arc};

use aleph_primitives::{AuthorityId, KEY_TYPE};
use codec::{Decode, Encode};
use futures::future::pending;
use log::info;
use sp_keystore::{testing::KeyStore, CryptoStore};
use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpListener, TcpStream, ToSocketAddrs,
};

use crate::{
    network::{Data, Multiaddress, NetworkIdentity, PeerId},
    validator_network::{Dialer, Listener, Network, Splittable},
};

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

/// Create a new tcp network, including an identity that can be used for constructing
/// authentications for other peers.
#[allow(dead_code)]
pub async fn new_tcp_network<A: ToSocketAddrs>(
    listening_addresses: A,
    external_addresses: Vec<String>,
    peer_id: AuthorityId,
) -> IoResult<(
    impl Dialer<TcpMultiaddress>,
    impl Listener,
    impl NetworkIdentity,
)> {
    let listener = TcpListener::bind(listening_addresses).await?;
    let identity = TcpNetworkIdentity {
        addresses: external_addresses
            .into_iter()
            .map(|address| TcpMultiaddress {
                peer_id: peer_id.clone(),
                address,
            })
            .collect(),
        peer_id,
    };
    Ok((TcpDialer {}, listener, identity))
}

/// This struct is for integration only. Will be removed after A0-1411.
struct NoopNetwork<D: Data> {
    _phantom: PhantomData<D>,
}

#[async_trait::async_trait]
impl<D: Data> Network<TcpMultiaddress, D> for NoopNetwork<D> {
    fn add_connection(&mut self, _peer: AuthorityId, _addresses: Vec<TcpMultiaddress>) {}

    fn remove_connection(&mut self, _peer: AuthorityId) {}

    fn send(&self, _data: D, _recipient: AuthorityId) {}

    async fn next(&mut self) -> Option<D> {
        Some(pending::<D>().await)
    }
}

pub async fn new_noop<D: Data>() -> (
    impl Network<TcpMultiaddress, D>,
    impl NetworkIdentity<Multiaddress = TcpMultiaddress, PeerId = AuthorityId>,
) {
    let key_store = Arc::new(KeyStore::new());
    let peer_id: AuthorityId = key_store
        .ed25519_generate_new(KEY_TYPE, None)
        .await
        .unwrap()
        .into();
    let addresses = vec![TcpMultiaddress {
        peer_id: peer_id.clone(),
        address: String::from(""),
    }];
    (
        NoopNetwork {
            _phantom: PhantomData,
        },
        TcpNetworkIdentity { peer_id, addresses },
    )
}
