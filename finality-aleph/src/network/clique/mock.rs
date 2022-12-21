use std::{
    collections::HashMap,
    fmt::{Display, Error as FmtError, Formatter},
    io::Result as IoResult,
    pin::Pin,
    task::{Context, Poll},
};

use codec::{Decode, Encode};
use futures::{
    channel::{mpsc, mpsc::UnboundedReceiver, oneshot},
    Future, StreamExt,
};
use log::info;
use rand::Rng;
use tokio::io::{duplex, AsyncRead, AsyncWrite, DuplexStream, ReadBuf};

use crate::network::{
    clique::{
        protocols::{ProtocolError, ResultForService},
        ConnectionInfo, Dialer, Listener, Network, PeerAddressInfo, PublicKey, SecretKey,
        Splittable, LOG_TARGET,
    },
    mock::Channel,
    AddressingInformation, Data, NetworkIdentity, PeerId,
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

#[derive(Clone, Debug, PartialEq, Eq, Hash, Encode, Decode)]
pub struct MockAddressingInformation {
    peer_id: MockPublicKey,
    address: String,
    valid: bool,
}

impl AddressingInformation for MockAddressingInformation {
    type PeerId = MockPublicKey;

    fn peer_id(&self) -> Self::PeerId {
        self.peer_id.clone()
    }

    fn verify(&self) -> bool {
        self.valid
    }
}

impl NetworkIdentity for MockAddressingInformation {
    type PeerId = MockPublicKey;
    type AddressingInformation = MockAddressingInformation;

    fn identity(&self) -> Self::AddressingInformation {
        self.clone()
    }
}

impl From<MockAddressingInformation> for Vec<MockAddressingInformation> {
    fn from(address: MockAddressingInformation) -> Self {
        vec![address]
    }
}

impl TryFrom<Vec<MockAddressingInformation>> for MockAddressingInformation {
    type Error = ();

    fn try_from(mut addresses: Vec<MockAddressingInformation>) -> Result<Self, Self::Error> {
        match addresses.pop() {
            Some(address) => Ok(address),
            None => Err(()),
        }
    }
}

pub fn random_peer_id() -> MockPublicKey {
    key().0
}

pub fn random_address_from(address: String, valid: bool) -> MockAddressingInformation {
    let peer_id = random_peer_id();
    MockAddressingInformation {
        peer_id,
        address,
        valid,
    }
}

pub fn random_address() -> MockAddressingInformation {
    random_address_from(
        rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .map(char::from)
            .take(43)
            .collect(),
        true,
    )
}

pub fn random_invalid_address() -> MockAddressingInformation {
    random_address_from(
        rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .map(char::from)
            .take(43)
            .collect(),
        false,
    )
}

#[derive(Clone)]
pub struct MockNetwork<D: Data> {
    pub add_connection: Channel<(MockPublicKey, MockAddressingInformation)>,
    pub remove_connection: Channel<MockPublicKey>,
    pub send: Channel<(D, MockPublicKey)>,
    pub next: Channel<D>,
}

#[async_trait::async_trait]
impl<D: Data> Network<MockPublicKey, MockAddressingInformation, D> for MockNetwork<D> {
    fn add_connection(&mut self, peer: MockPublicKey, address: MockAddressingInformation) {
        self.add_connection.send((peer, address));
    }

    fn remove_connection(&mut self, peer: MockPublicKey) {
        self.remove_connection.send(peer);
    }

    fn send(&self, data: D, recipient: MockPublicKey) {
        self.send.send((data, recipient));
    }

    async fn next(&mut self) -> Option<D> {
        self.next.next().await
    }
}

impl<D: Data> MockNetwork<D> {
    pub fn new() -> Self {
        MockNetwork {
            add_connection: Channel::new(),
            remove_connection: Channel::new(),
            send: Channel::new(),
            next: Channel::new(),
        }
    }

    // Consumes the network asserting there are no unreceived messages in the channels.
    pub async fn close_channels(self) {
        assert!(self.add_connection.close().await.is_none());
        assert!(self.remove_connection.close().await.is_none());
        assert!(self.send.close().await.is_none());
        assert!(self.next.close().await.is_none());
    }
}

impl<D: Data> Default for MockNetwork<D> {
    fn default() -> Self {
        Self::new()
    }
}

/// Bidirectional in-memory stream that closes abruptly after a specified
/// number of poll_write calls.
#[derive(Debug)]
pub struct UnreliableDuplexStream {
    stream: DuplexStream,
    counter: Option<usize>,
    peer_address: Address,
}

impl AsyncWrite for UnreliableDuplexStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<IoResult<usize>> {
        let this = self.get_mut();
        if let Some(ref mut c) = this.counter {
            if c == &0 {
                if Pin::new(&mut this.stream).poll_shutdown(cx).is_pending() {
                    return Poll::Pending;
                }
            } else {
                *c -= 1;
            }
        }
        Pin::new(&mut this.stream).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Pin::new(&mut self.get_mut().stream).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Pin::new(&mut self.get_mut().stream).poll_shutdown(cx)
    }
}

impl AsyncRead for UnreliableDuplexStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<IoResult<()>> {
        Pin::new(&mut self.get_mut().stream).poll_read(cx, buf)
    }
}

/// A stream that can be split into two instances of UnreliableDuplexStream.
#[derive(Debug)]
pub struct UnreliableSplittable {
    incoming_data: UnreliableDuplexStream,
    outgoing_data: UnreliableDuplexStream,
    peer_address: Address,
}

impl UnreliableSplittable {
    /// Create a pair of mock splittables connected to each other.
    pub fn new(
        max_buf_size: usize,
        ends_after: Option<usize>,
        l_address: Address,
        r_address: Address,
    ) -> (Self, Self) {
        let (l_in, r_out) = duplex(max_buf_size);
        let (r_in, l_out) = duplex(max_buf_size);
        (
            UnreliableSplittable {
                incoming_data: UnreliableDuplexStream {
                    stream: l_in,
                    counter: ends_after,
                    peer_address: r_address,
                },
                outgoing_data: UnreliableDuplexStream {
                    stream: l_out,
                    counter: ends_after,
                    peer_address: r_address,
                },
                peer_address: r_address,
            },
            UnreliableSplittable {
                incoming_data: UnreliableDuplexStream {
                    stream: r_in,
                    counter: ends_after,
                    peer_address: l_address,
                },
                outgoing_data: UnreliableDuplexStream {
                    stream: r_out,
                    counter: ends_after,
                    peer_address: l_address,
                },
                peer_address: l_address,
            },
        )
    }
}

impl AsyncRead for UnreliableSplittable {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<IoResult<()>> {
        Pin::new(&mut self.get_mut().incoming_data).poll_read(cx, buf)
    }
}

impl AsyncWrite for UnreliableSplittable {
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

impl ConnectionInfo for UnreliableSplittable {
    fn peer_address_info(&self) -> PeerAddressInfo {
        self.peer_address.to_string()
    }
}

impl ConnectionInfo for UnreliableDuplexStream {
    fn peer_address_info(&self) -> PeerAddressInfo {
        self.peer_address.to_string()
    }
}

impl Splittable for UnreliableSplittable {
    type Sender = UnreliableDuplexStream;
    type Receiver = UnreliableDuplexStream;

    fn split(self) -> (Self::Sender, Self::Receiver) {
        (self.outgoing_data, self.incoming_data)
    }
}

type Address = u32;
pub type Addresses = HashMap<MockPublicKey, Address>;
type Callers = HashMap<MockPublicKey, (MockDialer, MockListener)>;
type Connection = UnreliableSplittable;

#[derive(Clone)]
pub struct MockDialer {
    // used for logging
    own_address: Address,
    channel_connect: mpsc::UnboundedSender<(Address, Address, oneshot::Sender<Connection>)>,
}

#[async_trait::async_trait]
impl Dialer<Address> for MockDialer {
    type Connection = Connection;
    type Error = std::io::Error;

    async fn connect(&mut self, address: Address) -> Result<Self::Connection, Self::Error> {
        let (tx, rx) = oneshot::channel();
        self.channel_connect
            .unbounded_send((self.own_address, address, tx))
            .expect("should send");
        Ok(rx.await.expect("should receive"))
    }
}

pub struct MockListener {
    channel_accept: mpsc::UnboundedReceiver<Connection>,
}

#[async_trait::async_trait]
impl Listener for MockListener {
    type Connection = Connection;
    type Error = std::io::Error;

    async fn accept(&mut self) -> Result<Self::Connection, Self::Error> {
        Ok(self.channel_accept.next().await.expect("should receive"))
    }
}

pub struct UnreliableConnectionMaker {
    dialers: mpsc::UnboundedReceiver<(Address, Address, oneshot::Sender<Connection>)>,
    listeners: Vec<mpsc::UnboundedSender<Connection>>,
}

impl UnreliableConnectionMaker {
    pub fn new(ids: Vec<MockPublicKey>) -> (Self, Callers, Addresses) {
        let mut listeners = Vec::with_capacity(ids.len());
        let mut callers = HashMap::with_capacity(ids.len());
        let (tx_dialer, dialers) = mpsc::unbounded();
        // create peer addresses that will be understood by the main loop in method run
        // each peer gets a one-element vector containing its index, so we'll be able
        // to retrieve proper communication channels
        let addr: Addresses = ids
            .clone()
            .into_iter()
            .zip(0..ids.len())
            .map(|(id, u)| (id, u as u32))
            .collect();
        // create callers for every peer, keep channels for communicating with them
        for id in ids.into_iter() {
            let (tx_listener, rx_listener) = mpsc::unbounded();
            let dialer = MockDialer {
                own_address: *addr.get(&id).expect("should be there"),
                channel_connect: tx_dialer.clone(),
            };
            let listener = MockListener {
                channel_accept: rx_listener,
            };
            listeners.push(tx_listener);
            callers.insert(id, (dialer, listener));
        }
        (
            UnreliableConnectionMaker { dialers, listeners },
            callers,
            addr,
        )
    }

    pub async fn run(&mut self, connections_end_after: Option<usize>) {
        loop {
            info!(
                target: LOG_TARGET,
                "UnreliableConnectionMaker: waiting for new request..."
            );
            let (dialer_address, listener_address, c) =
                self.dialers.next().await.expect("should receive");
            info!(
                target: LOG_TARGET,
                "UnreliableConnectionMaker: received request"
            );
            let (dialer_stream, listener_stream) = Connection::new(
                4096,
                connections_end_after,
                dialer_address,
                listener_address,
            );
            info!(
                target: LOG_TARGET,
                "UnreliableConnectionMaker: sending stream"
            );
            c.send(dialer_stream).expect("should send");
            self.listeners[listener_address as usize]
                .unbounded_send(listener_stream)
                .expect("should send");
        }
    }
}

pub struct MockPrelims<D> {
    pub id_incoming: MockPublicKey,
    pub pen_incoming: MockSecretKey,
    pub id_outgoing: MockPublicKey,
    pub pen_outgoing: MockSecretKey,
    pub incoming_handle: Pin<Box<dyn Future<Output = Result<(), ProtocolError<MockPublicKey>>>>>,
    pub outgoing_handle: Pin<Box<dyn Future<Output = Result<(), ProtocolError<MockPublicKey>>>>>,
    pub data_from_incoming: UnboundedReceiver<D>,
    pub data_from_outgoing: Option<UnboundedReceiver<D>>,
    pub result_from_incoming: UnboundedReceiver<ResultForService<MockPublicKey, D>>,
    pub result_from_outgoing: UnboundedReceiver<ResultForService<MockPublicKey, D>>,
}
