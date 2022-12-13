use std::{
    collections::HashMap,
    fmt::{Display, Error as FmtError, Formatter},
    io::Result as IoResult,
    pin::Pin,
    task::{Context, Poll},
};

use codec::{Decode, Encode};
use tokio::io::{duplex, AsyncRead, AsyncWrite, DuplexStream, ReadBuf};

use crate::{
    network::PeerId,
    validator_network::{ConnectionInfo, PeerAddressInfo, PublicKey, SecretKey, Splittable},
};

/// A mock secret key that is able to sign messages.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct MockSecretKey([u8; 4]);

/// A mock public key for verifying signatures.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Encode, Decode)]
pub struct MockPublicKey([u8; 4]);

impl Display for MockPublicKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(f, "PublicKey({:?})", self.0)
    }
}

impl AsRef<[u8]> for MockPublicKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

/// A mock signature, able to discern whether the correct key has been used to sign a specific
/// message.
#[derive(Debug, PartialEq, Eq, Clone, Hash, Encode, Decode)]
pub struct MockSignature {
    message: Vec<u8>,
    key_id: [u8; 4],
}

impl PublicKey for MockPublicKey {
    type Signature = MockSignature;

    fn verify(&self, message: &[u8], signature: &Self::Signature) -> bool {
        (message == signature.message.as_slice()) && (self.0 == signature.key_id)
    }
}

impl PeerId for MockPublicKey {}

#[async_trait::async_trait]
impl SecretKey for MockSecretKey {
    type Signature = MockSignature;
    type PublicKey = MockPublicKey;

    async fn sign(&self, message: &[u8]) -> Self::Signature {
        MockSignature {
            message: message.to_vec(),
            key_id: self.0,
        }
    }

    fn public_key(&self) -> Self::PublicKey {
        MockPublicKey(self.0)
    }
}

/// Create a random key pair.
pub fn key() -> (MockPublicKey, MockSecretKey) {
    let secret_key = MockSecretKey(rand::random());
    (secret_key.public_key(), secret_key)
}

/// Create a HashMap with public keys as keys and secret keys as values.
pub fn random_keys(n_peers: usize) -> HashMap<MockPublicKey, MockSecretKey> {
    let mut result = HashMap::with_capacity(n_peers);
    while result.len() < n_peers {
        let (pk, sk) = key();
        result.insert(pk, sk);
    }
    result
}

/// A mock that can be split into two streams.
pub struct MockSplittable {
    incoming_data: DuplexStream,
    outgoing_data: DuplexStream,
}

impl MockSplittable {
    /// Create a pair of mock splittables connected to each other.
    pub fn new(max_buf_size: usize) -> (Self, Self) {
        let (in_a, out_b) = duplex(max_buf_size);
        let (in_b, out_a) = duplex(max_buf_size);
        (
            MockSplittable {
                incoming_data: in_a,
                outgoing_data: out_a,
            },
            MockSplittable {
                incoming_data: in_b,
                outgoing_data: out_b,
            },
        )
    }
}

impl AsyncRead for MockSplittable {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<IoResult<()>> {
        Pin::new(&mut self.get_mut().incoming_data).poll_read(cx, buf)
    }
}

impl AsyncWrite for MockSplittable {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<IoResult<usize>> {
        Pin::new(&mut self.get_mut().outgoing_data).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Pin::new(&mut self.get_mut().outgoing_data).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Pin::new(&mut self.get_mut().outgoing_data).poll_shutdown(cx)
    }
}

impl ConnectionInfo for MockSplittable {
    fn peer_address_info(&self) -> PeerAddressInfo {
        String::from("MOCK_ADDRESS")
    }
}

impl ConnectionInfo for DuplexStream {
    fn peer_address_info(&self) -> PeerAddressInfo {
        String::from("MOCK_ADDRESS")
    }
}

impl Splittable for MockSplittable {
    type Sender = DuplexStream;
    type Receiver = DuplexStream;

    fn split(self) -> (Self::Sender, Self::Receiver) {
        (self.outgoing_data, self.incoming_data)
    }
}
