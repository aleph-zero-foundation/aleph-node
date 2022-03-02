use crate::network::{
    ConnectionCommand, Data, DataCommand, Network, NetworkEventStream, NetworkSender, PeerId, IO,
};
use async_trait::async_trait;
use futures::{
    channel::{mpsc, oneshot},
    StreamExt,
};
use parking_lot::Mutex;
use sc_network::{Event, Multiaddr};
use std::{
    borrow::Cow,
    collections::{HashSet, VecDeque},
    fmt,
    sync::Arc,
};

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

pub struct MockIO<D: Data> {
    pub messages_for_user: mpsc::UnboundedSender<(D, DataCommand)>,
    pub messages_from_user: mpsc::UnboundedReceiver<D>,
    pub commands_for_manager: mpsc::UnboundedSender<ConnectionCommand>,
}

impl<D: Data> MockIO<D> {
    pub fn new() -> (MockIO<D>, IO<D>) {
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

pub struct MockNetworkSender<D: Data> {
    sender: mpsc::UnboundedSender<(D, PeerId, Cow<'static, str>)>,
    peer_id: PeerId,
    protocol: Cow<'static, str>,
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
                self.protocol.clone(),
            ))
            .unwrap();
        Ok(())
    }
}

#[derive(Clone)]
pub struct MockNetwork<D: Data> {
    pub add_reserved: Channel<(HashSet<Multiaddr>, Cow<'static, str>)>,
    pub remove_reserved: Channel<(HashSet<PeerId>, Cow<'static, str>)>,
    pub send_message: Channel<(D, PeerId, Cow<'static, str>)>,
    pub event_sinks: Arc<Mutex<Vec<mpsc::UnboundedSender<Event>>>>,
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

    fn event_stream(&self) -> NetworkEventStream {
        let (tx, rx) = mpsc::unbounded();
        self.event_sinks.lock().push(tx);
        // Necessary for tests to detect when service takes event_stream
        if let Some(tx) = self.event_stream_taken_oneshot.lock().take() {
            tx.send(()).unwrap();
        }
        Box::pin(rx)
    }

    fn sender(
        &self,
        peer_id: PeerId,
        protocol: Cow<'static, str>,
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

    fn add_reserved(&self, addresses: HashSet<Multiaddr>, protocol: Cow<'static, str>) {
        self.add_reserved.send((addresses, protocol));
    }

    fn remove_reserved(&self, peers: HashSet<PeerId>, protocol: Cow<'static, str>) {
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

    pub fn emit_event(&mut self, event: Event) {
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
