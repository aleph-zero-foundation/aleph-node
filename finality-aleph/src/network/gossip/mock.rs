use std::{collections::VecDeque, fmt, sync::Arc};

use async_trait::async_trait;
use futures::{
    channel::{mpsc, oneshot},
    StreamExt,
};
use network_clique::mock::MockPublicKey;
use parking_lot::Mutex;

use crate::network::{
    gossip::{Event, EventStream, NetworkSender, Protocol},
    mock::Channel,
};

pub type MockEvent = Event<MockPublicKey>;

pub struct MockEventStream(mpsc::UnboundedReceiver<MockEvent>);

#[async_trait]
impl EventStream<MockPublicKey> for MockEventStream {
    async fn next_event(&mut self) -> Option<MockEvent> {
        self.0.next().await
    }
}

pub struct MockNetworkSender {
    sender: mpsc::UnboundedSender<(Vec<u8>, MockPublicKey, Protocol)>,
    peer_id: MockPublicKey,
    protocol: Protocol,
    error: Result<(), MockSenderError>,
}

#[async_trait]
impl NetworkSender for MockNetworkSender {
    type SenderError = MockSenderError;

    async fn send<'a>(
        &'a self,
        data: impl Into<Vec<u8>> + Send + Sync + 'static,
    ) -> Result<(), MockSenderError> {
        self.error?;
        self.sender
            .unbounded_send((data.into(), self.peer_id.clone(), self.protocol))
            .unwrap();
        Ok(())
    }
}

#[derive(Clone)]
pub struct MockRawNetwork {
    pub send_message: Channel<(Vec<u8>, MockPublicKey, Protocol)>,
    pub event_sinks: Arc<Mutex<Vec<mpsc::UnboundedSender<MockEvent>>>>,
    event_stream_taken_oneshot: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    pub create_sender_errors: Arc<Mutex<VecDeque<MockSenderError>>>,
    pub send_errors: Arc<Mutex<VecDeque<MockSenderError>>>,
}

#[derive(Debug, Copy, Clone)]
pub struct MockSenderError;

impl fmt::Display for MockSenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Some error message")
    }
}

impl std::error::Error for MockSenderError {}

impl MockRawNetwork {
    pub fn new(oneshot_sender: oneshot::Sender<()>) -> Self {
        MockRawNetwork {
            send_message: Channel::new(),
            event_sinks: Arc::new(Mutex::new(vec![])),
            event_stream_taken_oneshot: Arc::new(Mutex::new(Some(oneshot_sender))),
            create_sender_errors: Arc::new(Mutex::new(VecDeque::new())),
            send_errors: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn event_stream(&self) -> MockEventStream {
        let (tx, rx) = mpsc::unbounded();
        self.event_sinks.lock().push(tx);
        // Necessary for tests to detect when service takes event_stream
        if let Some(tx) = self.event_stream_taken_oneshot.lock().take() {
            tx.send(()).unwrap();
        }
        MockEventStream(rx)
    }

    // Consumes the network asserting there are no unreceived messages in the channels.
    pub async fn close_channels(self) {
        self.event_sinks.lock().clear();
        // We disable it until tests regarding new substrate network protocol are created.
        // assert!(self.add_reserved.close().await.is_none());
        // assert!(self.remove_reserved.close().await.is_none());
        assert!(self.send_message.close().await.is_none());
    }
}
