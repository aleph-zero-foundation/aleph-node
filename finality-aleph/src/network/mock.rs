use std::{
    collections::{HashSet, VecDeque},
    fmt,
    sync::Arc,
};

use aleph_primitives::KEY_TYPE;
use async_trait::async_trait;
use codec::{Decode, Encode};
use futures::{
    channel::{mpsc, oneshot},
    StreamExt,
};
use parking_lot::Mutex;
use rand::random;
use sp_keystore::{testing::KeyStore, CryptoStore};

use crate::{
    crypto::{AuthorityPen, AuthorityVerifier},
    network::{
        ConnectionCommand, Data, DataCommand, Event, EventStream, Multiaddress, Network,
        NetworkIdentity, NetworkSender, PeerId, Protocol, IO,
    },
    AuthorityId, NodeIndex,
};

#[derive(PartialEq, Eq, Copy, Clone, Debug, Hash, Encode, Decode)]
pub struct MockPeerId(u32);

impl MockPeerId {
    pub fn random() -> Self {
        MockPeerId(random())
    }
}
impl fmt::Display for MockPeerId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PeerId for MockPeerId {}

#[derive(PartialEq, Eq, Clone, Debug, Hash, Encode, Decode)]
pub struct MockMultiaddress {
    peer_id: Option<MockPeerId>,
    address: u32,
}

impl MockMultiaddress {
    pub fn random_with_id(peer_id: MockPeerId) -> Self {
        MockMultiaddress {
            peer_id: Some(peer_id),
            address: random(),
        }
    }
}

impl Multiaddress for MockMultiaddress {
    type PeerId = MockPeerId;

    fn get_peer_id(&self) -> Option<Self::PeerId> {
        self.peer_id
    }

    fn add_matching_peer_id(mut self, peer_id: Self::PeerId) -> Option<Self> {
        match self.peer_id {
            Some(old_peer_id) => match old_peer_id == peer_id {
                true => Some(self),
                false => None,
            },
            None => {
                self.peer_id = Some(peer_id);
                Some(self)
            }
        }
    }
}

pub struct MockNetworkIdentity {
    addresses: Vec<MockMultiaddress>,
    peer_id: MockPeerId,
}

impl MockNetworkIdentity {
    pub fn new() -> Self {
        let peer_id = MockPeerId::random();
        let addresses = (0..3)
            .map(|_| MockMultiaddress::random_with_id(peer_id))
            .collect();
        MockNetworkIdentity { addresses, peer_id }
    }
}

impl NetworkIdentity for MockNetworkIdentity {
    type PeerId = MockPeerId;
    type Multiaddress = MockMultiaddress;

    fn identity(&self) -> (Vec<Self::Multiaddress>, Self::PeerId) {
        (self.addresses.clone(), self.peer_id)
    }
}

#[derive(Clone)]
pub struct Channel<T>(
    pub mpsc::UnboundedSender<T>,
    pub Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<T>>>,
);

impl<T> Channel<T> {
    fn new() -> Self {
        let (tx, rx) = mpsc::unbounded();
        Channel(tx, Arc::new(tokio::sync::Mutex::new(rx)))
    }

    fn send(&self, msg: T) {
        self.0.unbounded_send(msg).unwrap();
    }

    pub async fn next(&mut self) -> Option<T> {
        self.1.lock().await.next().await
    }

    pub async fn try_next(&self) -> Option<T> {
        self.1.lock().await.try_next().unwrap_or(None)
    }

    async fn close(self) -> Option<T> {
        self.0.close_channel();
        self.try_next().await
    }
}

pub type MockDataCommand = DataCommand<MockPeerId>;
pub type MockConnectionCommand = ConnectionCommand<MockMultiaddress>;
pub type MockEvent = Event<MockMultiaddress>;

pub type MockData = Vec<u8>;

pub struct MockIO {
    pub messages_for_user: mpsc::UnboundedSender<(MockData, MockDataCommand)>,
    pub messages_from_user: mpsc::UnboundedReceiver<MockData>,
    pub commands_for_manager: mpsc::UnboundedSender<MockConnectionCommand>,
}

impl MockIO {
    pub fn new() -> (MockIO, IO<MockData, MockMultiaddress>) {
        let (mock_messages_for_user, messages_from_user) = mpsc::unbounded();
        let (messages_for_user, mock_messages_from_user) = mpsc::unbounded();
        let (mock_commands_for_manager, commands_from_manager) = mpsc::unbounded();
        (
            MockIO {
                messages_for_user: mock_messages_for_user,
                messages_from_user: mock_messages_from_user,
                commands_for_manager: mock_commands_for_manager,
            },
            IO::new(messages_from_user, messages_for_user, commands_from_manager),
        )
    }
}

pub struct MockEventStream(mpsc::UnboundedReceiver<MockEvent>);

#[async_trait]
impl EventStream<MockMultiaddress> for MockEventStream {
    async fn next_event(&mut self) -> Option<MockEvent> {
        self.0.next().await
    }
}

pub struct MockNetworkSender<D: Data> {
    sender: mpsc::UnboundedSender<(D, MockPeerId, Protocol)>,
    peer_id: MockPeerId,
    protocol: Protocol,
    error: Result<(), MockSenderError>,
}

#[async_trait]
impl<D: Data> NetworkSender for MockNetworkSender<D> {
    type SenderError = MockSenderError;

    async fn send<'a>(
        &'a self,
        data: impl Into<Vec<u8>> + Send + Sync + 'static,
    ) -> Result<(), MockSenderError> {
        self.error?;
        self.sender
            .unbounded_send((
                D::decode(&mut &data.into()[..]).unwrap(),
                self.peer_id,
                self.protocol,
            ))
            .unwrap();
        Ok(())
    }
}

#[derive(Clone)]
pub struct MockNetwork<D: Data> {
    pub add_reserved: Channel<(HashSet<MockMultiaddress>, Protocol)>,
    pub remove_reserved: Channel<(HashSet<MockPeerId>, Protocol)>,
    pub send_message: Channel<(D, MockPeerId, Protocol)>,
    pub event_sinks: Arc<Mutex<Vec<mpsc::UnboundedSender<MockEvent>>>>,
    event_stream_taken_oneshot: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    pub create_sender_errors: Arc<Mutex<VecDeque<MockSenderError>>>,
    pub send_errors: Arc<Mutex<VecDeque<MockSenderError>>>,
}

#[derive(Debug, Copy, Clone)]
pub enum MockSenderError {
    SomeError,
}

impl fmt::Display for MockSenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MockSenderError::SomeError => {
                write!(f, "Some error message")
            }
        }
    }
}

impl std::error::Error for MockSenderError {}

impl<D: Data> Network for MockNetwork<D> {
    type SenderError = MockSenderError;
    type NetworkSender = MockNetworkSender<D>;
    type PeerId = MockPeerId;
    type Multiaddress = MockMultiaddress;
    type EventStream = MockEventStream;

    fn event_stream(&self) -> Self::EventStream {
        let (tx, rx) = mpsc::unbounded();
        self.event_sinks.lock().push(tx);
        // Necessary for tests to detect when service takes event_stream
        if let Some(tx) = self.event_stream_taken_oneshot.lock().take() {
            tx.send(()).unwrap();
        }
        MockEventStream(rx)
    }

    fn sender(
        &self,
        peer_id: Self::PeerId,
        protocol: Protocol,
    ) -> Result<Self::NetworkSender, Self::SenderError> {
        self.create_sender_errors
            .lock()
            .pop_front()
            .map_or(Ok(()), Err)?;
        let error = self.send_errors.lock().pop_front().map_or(Ok(()), Err);
        Ok(MockNetworkSender {
            sender: self.send_message.0.clone(),
            peer_id,
            protocol,
            error,
        })
    }

    fn add_reserved(&self, addresses: HashSet<Self::Multiaddress>, protocol: Protocol) {
        self.add_reserved.send((addresses, protocol));
    }

    fn remove_reserved(&self, peers: HashSet<Self::PeerId>, protocol: Protocol) {
        self.remove_reserved.send((peers, protocol));
    }
}

impl<D: Data> MockNetwork<D> {
    pub fn new(oneshot_sender: oneshot::Sender<()>) -> Self {
        MockNetwork {
            add_reserved: Channel::new(),
            remove_reserved: Channel::new(),
            send_message: Channel::new(),
            event_sinks: Arc::new(Mutex::new(vec![])),
            event_stream_taken_oneshot: Arc::new(Mutex::new(Some(oneshot_sender))),
            create_sender_errors: Arc::new(Mutex::new(VecDeque::new())),
            send_errors: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn emit_event(&mut self, event: MockEvent) {
        for sink in &*self.event_sinks.lock() {
            sink.unbounded_send(event.clone()).unwrap();
        }
    }

    // Consumes the network asserting there are no unreceived messages in the channels.
    pub async fn close_channels(self) {
        self.event_sinks.lock().clear();
        assert!(self.add_reserved.close().await.is_none());
        assert!(self.remove_reserved.close().await.is_none());
        assert!(self.send_message.close().await.is_none());
    }
}

pub async fn crypto_basics(
    num_crypto_basics: usize,
) -> (Vec<(NodeIndex, AuthorityPen)>, AuthorityVerifier) {
    let keystore = Arc::new(KeyStore::new());
    let mut auth_ids = Vec::with_capacity(num_crypto_basics);
    for _ in 0..num_crypto_basics {
        let pk = keystore.ed25519_generate_new(KEY_TYPE, None).await.unwrap();
        auth_ids.push(AuthorityId::from(pk));
    }
    let mut result = Vec::with_capacity(num_crypto_basics);
    for (i, auth_id) in auth_ids.iter().enumerate() {
        result.push((
            NodeIndex(i),
            AuthorityPen::new(auth_id.clone(), keystore.clone())
                .await
                .expect("The keys should sign successfully"),
        ));
    }
    (result, AuthorityVerifier::new(auth_ids))
}
