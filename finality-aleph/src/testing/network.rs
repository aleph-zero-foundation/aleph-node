use crate::{
    crypto::{AuthorityPen, AuthorityVerifier, KeyBox},
    network::{
        AuthData, ConsensusNetwork, DataNetwork, InternalMessage, MetaMessage, Network, PeerId,
        Recipient,
    },
    AuthorityId, SessionId,
};
use aleph_bft::{Index, KeyBox as _, NodeIndex};
use aleph_primitives::KEY_TYPE;
use codec::DecodeAll;
use futures::{
    channel::{mpsc, oneshot},
    stream::Stream,
    StreamExt,
};
use parking_lot::Mutex;
use sc_network::{Event, ObservedRole, PeerId as ScPeerId, ReputationChange};
use sp_api::NumberFor;
use sp_core::Encode;
use sp_keystore::{testing::KeyStore, CryptoStore};
use sp_runtime::traits::Block as BlockT;
use std::{borrow::Cow, pin::Pin, sync::Arc};
use substrate_test_runtime::Block;

type Channel<T> = (
    Arc<Mutex<mpsc::UnboundedSender<T>>>,
    Arc<Mutex<mpsc::UnboundedReceiver<T>>>,
);

fn channel<T>() -> Channel<T> {
    let (tx, rx) = mpsc::unbounded();
    (Arc::new(Mutex::new(tx)), Arc::new(Mutex::new(rx)))
}

#[derive(Clone)]
struct TestNetwork<B: BlockT> {
    event_sinks: Arc<Mutex<Vec<mpsc::UnboundedSender<Event>>>>,
    oneshot_sender: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    report_peer: Channel<(PeerId, ReputationChange)>,
    disconnect_peer: Channel<(PeerId, Cow<'static, str>)>,
    send_message: Channel<(PeerId, Cow<'static, str>, Vec<u8>)>,
    announce: Channel<(B::Hash, Option<Vec<u8>>)>,
    add_set_reserved: Channel<(PeerId, Cow<'static, str>)>,
    remove_set_reserved: Channel<(PeerId, Cow<'static, str>)>,
    request_justification: Channel<(B::Hash, NumberFor<B>)>,
    peer_id: PeerId,
}

impl<B: BlockT> TestNetwork<B> {
    fn new(peer_id: PeerId, tx: oneshot::Sender<()>) -> Self {
        TestNetwork {
            event_sinks: Arc::new(Mutex::new(vec![])),
            oneshot_sender: Arc::new(Mutex::new(Some(tx))),
            report_peer: channel(),
            disconnect_peer: channel(),
            send_message: channel(),
            announce: channel(),
            add_set_reserved: channel(),
            remove_set_reserved: channel(),
            request_justification: channel(),
            peer_id,
        }
    }
}

impl<B: BlockT> Network<B> for TestNetwork<B> {
    fn event_stream(&self) -> Pin<Box<dyn Stream<Item = Event> + Send>> {
        let (tx, rx) = mpsc::unbounded();
        self.event_sinks.lock().push(tx);
        if let Some(tx) = self.oneshot_sender.lock().take() {
            tx.send(()).unwrap();
        }
        Box::pin(rx)
    }

    fn _report_peer(&self, peer_id: PeerId, reputation: ReputationChange) {
        self.report_peer
            .0
            .lock()
            .unbounded_send((peer_id, reputation))
            .unwrap();
    }

    fn _disconnect_peer(&self, peer_id: PeerId, protocol: Cow<'static, str>) {
        self.disconnect_peer
            .0
            .lock()
            .unbounded_send((peer_id, protocol))
            .unwrap();
    }

    fn send_message(&self, peer_id: PeerId, protocol: Cow<'static, str>, message: Vec<u8>) {
        self.send_message
            .0
            .lock()
            .unbounded_send((peer_id, protocol, message))
            .unwrap();
    }

    fn _announce(&self, block: <B as BlockT>::Hash, associated_data: Option<Vec<u8>>) {
        self.announce
            .0
            .lock()
            .unbounded_send((block, associated_data))
            .unwrap();
    }

    fn add_set_reserved(&self, who: PeerId, protocol: Cow<'static, str>) {
        self.add_set_reserved
            .0
            .lock()
            .unbounded_send((who, protocol))
            .unwrap();
    }

    fn remove_set_reserved(&self, who: PeerId, protocol: Cow<'static, str>) {
        self.remove_set_reserved
            .0
            .lock()
            .unbounded_send((who, protocol))
            .unwrap();
    }

    fn peer_id(&self) -> PeerId {
        self.peer_id
    }
}

impl<B: BlockT> TestNetwork<B> {
    fn emit_event(&self, event: Event) {
        for sink in &*self.event_sinks.lock() {
            sink.unbounded_send(event.clone()).unwrap();
        }
    }

    // Consumes the network asserting there are no unreceived messages in the channels.
    fn close_channels(self) {
        self.event_sinks.lock().clear();
        self.report_peer.0.lock().close_channel();
        assert!(self.report_peer.1.lock().try_next().unwrap().is_none());
        self.disconnect_peer.0.lock().close_channel();
        assert!(self.disconnect_peer.1.lock().try_next().unwrap().is_none());
        self.send_message.0.lock().close_channel();
        assert!(self.send_message.1.lock().try_next().unwrap().is_none());
        self.announce.0.lock().close_channel();
        assert!(self.announce.1.lock().try_next().unwrap().is_none());
        self.add_set_reserved.0.lock().close_channel();
        assert!(self.add_set_reserved.1.lock().try_next().unwrap().is_none());
        self.remove_set_reserved.0.lock().close_channel();
        assert!(self
            .remove_set_reserved
            .1
            .lock()
            .try_next()
            .unwrap()
            .is_none());
    }
}

struct Authority {
    peer_id: PeerId,
    keychain: KeyBox,
}

async fn generate_authorities(ss: &[String]) -> Vec<Authority> {
    let key_store = Arc::new(KeyStore::new());
    let mut auth_ids = Vec::with_capacity(ss.len());
    for s in ss {
        let pk = key_store
            .ed25519_generate_new(KEY_TYPE, Some(s))
            .await
            .unwrap();
        auth_ids.push(AuthorityId::from(pk));
    }
    let mut result = Vec::with_capacity(ss.len());
    for i in 0..ss.len() {
        let keychain = KeyBox::new(
            NodeIndex(i),
            AuthorityVerifier::new(auth_ids.clone()),
            AuthorityPen::new(auth_ids[i].clone(), key_store.clone())
                .await
                .expect("The keys should sign successfully"),
        );
        result.push(Authority {
            peer_id: ScPeerId::random().into(),
            keychain,
        });
    }
    assert_eq!(key_store.keys(KEY_TYPE).await.unwrap().len(), 3 * ss.len());
    result
}

type MockData = Vec<u8>;

struct TestData {
    network: TestNetwork<Block>,
    authorities: Vec<Authority>,
    consensus_network_handle: tokio::task::JoinHandle<()>,
    data_network: DataNetwork<MockData>,
}

impl TestData {
    // consumes the test data asserting there are no unread messages in the channels
    // and awaits for the consensus_network task.
    async fn complete(mut self) {
        self.network.close_channels();
        assert!(self.data_network.next().await.is_none());
        self.consensus_network_handle.await.unwrap();
    }
}

const PROTOCOL_NAME: &str = "/test/1";

async fn prepare_one_session_test_data() -> TestData {
    let authority_names: Vec<_> = ["//Alice", "//Bob", "//Charlie"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let authorities = generate_authorities(authority_names.as_slice()).await;
    let peer_id = authorities[0].peer_id;

    let (oneshot_tx, oneshot_rx) = oneshot::channel();
    let network = TestNetwork::<Block>::new(peer_id, oneshot_tx);
    let consensus_network = ConsensusNetwork::<MockData, Block, TestNetwork<Block>>::new(
        network.clone(),
        PROTOCOL_NAME.into(),
    );

    let session_id = SessionId(0);

    let data_network = consensus_network
        .session_manager()
        .start_session(session_id, authorities[0].keychain.clone())
        .await;
    let consensus_network_handle = tokio::spawn(async move { consensus_network.run().await });

    // wait till consensus_network takes the event_stream
    oneshot_rx.await.unwrap();

    TestData {
        network,
        authorities,
        consensus_network_handle,
        data_network,
    }
}

#[tokio::test]
async fn test_network_event_sync_connnected() {
    let data = prepare_one_session_test_data().await;
    let bob_peer_id = data.authorities[1].peer_id;
    data.network.emit_event(Event::SyncConnected {
        remote: bob_peer_id.into(),
    });
    let (peer_id, protocol) = data.network.add_set_reserved.1.lock().next().await.unwrap();
    assert_eq!(peer_id, bob_peer_id);
    assert_eq!(protocol, PROTOCOL_NAME);
    data.complete().await;
}

#[tokio::test]
async fn test_network_event_sync_disconnected() {
    let data = prepare_one_session_test_data().await;
    let charlie_peer_id = data.authorities[2].peer_id;
    data.network.emit_event(Event::SyncDisconnected {
        remote: charlie_peer_id.into(),
    });
    let (peer_id, protocol) = data
        .network
        .remove_set_reserved
        .1
        .lock()
        .next()
        .await
        .unwrap();
    assert_eq!(peer_id, charlie_peer_id);
    assert_eq!(protocol, PROTOCOL_NAME);
    data.complete().await;
}

#[tokio::test]
async fn authenticates_to_connected() {
    let data = prepare_one_session_test_data().await;
    let bob_peer_id = data.authorities[1].peer_id;
    data.network.emit_event(Event::NotificationStreamOpened {
        remote: bob_peer_id.into(),
        protocol: Cow::Borrowed(PROTOCOL_NAME),
        role: ObservedRole::Authority,
        negotiated_fallback: None,
    });
    let (peer_id, protocol, message) = data
        .network
        .send_message
        .1
        .lock()
        .next()
        .await
        .expect("got auth message");
    let alice_peer_id = data.authorities[0].peer_id;
    assert_eq!(peer_id, bob_peer_id);
    assert_eq!(protocol, PROTOCOL_NAME);
    let message =
        InternalMessage::<MockData>::decode_all(message.as_slice()).expect("a correct message");
    if let InternalMessage::Meta(MetaMessage::Authentication(auth_data, _)) = message {
        assert_eq!(auth_data.peer_id, alice_peer_id);
    } else {
        panic!("Expected an authentication message.")
    }
    data.complete().await;
}

#[tokio::test]
async fn authenticates_when_requested() {
    let data = prepare_one_session_test_data().await;
    let bob_peer_id = data.authorities[1].peer_id;
    let auth_message =
        InternalMessage::<MockData>::Meta(MetaMessage::AuthenticationRequest(SessionId(0)))
            .encode();
    let messages = vec![(PROTOCOL_NAME.into(), auth_message.into())];

    data.network.emit_event(Event::NotificationsReceived {
        remote: bob_peer_id.into(),
        messages,
    });
    let (peer_id, protocol, message) = data
        .network
        .send_message
        .1
        .lock()
        .next()
        .await
        .expect("got auth message");
    let alice_peer_id = data.authorities[0].peer_id;
    assert_eq!(peer_id, bob_peer_id);
    assert_eq!(protocol, PROTOCOL_NAME);
    let message =
        InternalMessage::<MockData>::decode_all(message.as_slice()).expect("a correct message");
    if let InternalMessage::Meta(MetaMessage::Authentication(auth_data, _)) = message {
        assert_eq!(auth_data.peer_id, alice_peer_id);
    } else {
        panic!("Expected an authentication message.")
    }
    data.complete().await;
}

#[tokio::test]
async fn test_network_event_notifications_received() {
    let mut data = prepare_one_session_test_data().await;
    let bob_peer_id = data.authorities[1].peer_id;
    let bob_node_id = data.authorities[1].keychain.index();
    let auth_data = AuthData {
        session_id: SessionId(0),
        peer_id: bob_peer_id,
        node_id: bob_node_id,
    };
    let signature = data.authorities[1].keychain.sign(&auth_data.encode()).await;
    let auth_message =
        InternalMessage::<MockData>::Meta(MetaMessage::Authentication(auth_data, signature))
            .encode();
    let note = vec![157];
    let message = InternalMessage::Data(SessionId(0), note.clone()).encode();
    let messages = vec![
        (PROTOCOL_NAME.into(), auth_message.into()),
        (PROTOCOL_NAME.into(), message.clone().into()),
    ];

    data.network.emit_event(Event::NotificationsReceived {
        remote: bob_peer_id.into(),
        messages,
    });
    if let Some(incoming_data) = data.data_network.next().await {
        assert_eq!(incoming_data, note);
    } else {
        panic!("expected message received nothing")
    }
    data.complete().await;
}

#[tokio::test]
async fn requests_authentication_from_unauthenticated() {
    let data = prepare_one_session_test_data().await;
    let bob_peer_id = data.authorities[1].peer_id;
    let cur_session_id = SessionId(0);
    let note = vec![157];
    let message = InternalMessage::Data(cur_session_id, note).encode();
    let messages = vec![(PROTOCOL_NAME.into(), message.into())];

    data.network.emit_event(Event::NotificationsReceived {
        remote: bob_peer_id.into(),
        messages,
    });
    let (peer_id, protocol, message) = data
        .network
        .send_message
        .1
        .lock()
        .next()
        .await
        .expect("got auth request");
    assert_eq!(peer_id, bob_peer_id);
    assert_eq!(protocol, PROTOCOL_NAME);
    let message =
        InternalMessage::<MockData>::decode_all(message.as_slice()).expect("a correct message");
    if let InternalMessage::Meta(MetaMessage::AuthenticationRequest(session_id)) = message {
        assert_eq!(session_id, cur_session_id);
    } else {
        panic!("Expected an authentication request.")
    }
    data.complete().await;
}

#[tokio::test]
async fn test_send() {
    let data = prepare_one_session_test_data().await;
    let bob_peer_id = data.authorities[1].peer_id;
    let bob_node_id = data.authorities[1].keychain.index();
    let cur_session_id = SessionId(0);
    let auth_data = AuthData {
        session_id: cur_session_id,
        peer_id: bob_peer_id,
        node_id: bob_node_id,
    };
    let signature = data.authorities[1].keychain.sign(&auth_data.encode()).await;
    let auth_message =
        InternalMessage::<MockData>::Meta(MetaMessage::Authentication(auth_data, signature))
            .encode();
    let messages = vec![(PROTOCOL_NAME.into(), auth_message.into())];

    data.network.emit_event(Event::NotificationsReceived {
        remote: bob_peer_id.into(),
        messages,
    });
    data.network.emit_event(Event::NotificationStreamOpened {
        remote: bob_peer_id.into(),
        protocol: Cow::Borrowed(PROTOCOL_NAME),
        role: ObservedRole::Authority,
        negotiated_fallback: None,
    });
    // Wait for acknowledgement that Alice noted Bob's presence.
    data.network
        .send_message
        .1
        .lock()
        .next()
        .await
        .expect("got auth message");
    let note = vec![157];
    data.data_network
        .send(note.clone(), Recipient::Target(bob_node_id))
        .expect("sending works");
    match data.network.send_message.1.lock().next().await {
        Some((peer_id, protocol, message)) => {
            assert_eq!(peer_id, bob_peer_id);
            assert_eq!(protocol, PROTOCOL_NAME);
            match InternalMessage::<MockData>::decode_all(message.as_slice()) {
                Ok(InternalMessage::Data(session_id, data)) => {
                    assert_eq!(session_id, cur_session_id);
                    assert_eq!(data, note);
                }
                _ => panic!("Expected a properly encoded message"),
            }
        }
        _ => panic!("Expecting a message"),
    }
    data.complete().await;
}

#[tokio::test]
async fn test_broadcast() {
    let data = prepare_one_session_test_data().await;
    let cur_session_id = SessionId(0);
    for i in 1..2 {
        let peer_id = data.authorities[i].peer_id;
        let node_id = data.authorities[i].keychain.index();
        let auth_data = AuthData {
            session_id: cur_session_id,
            peer_id,
            node_id,
        };
        let signature = data.authorities[1].keychain.sign(&auth_data.encode()).await;
        let auth_message =
            InternalMessage::<MockData>::Meta(MetaMessage::Authentication(auth_data, signature))
                .encode();
        let messages = vec![(PROTOCOL_NAME.into(), auth_message.into())];

        data.network.emit_event(Event::NotificationsReceived {
            remote: peer_id.0,
            messages,
        });
        data.network.emit_event(Event::NotificationStreamOpened {
            remote: peer_id.0,
            protocol: Cow::Borrowed(PROTOCOL_NAME),
            role: ObservedRole::Authority,
            negotiated_fallback: None,
        });
        // Wait for acknowledgement that Alice noted the nodes presence.
        data.network
            .send_message
            .1
            .lock()
            .next()
            .await
            .expect("got auth message");
    }
    let note = vec![157];
    data.data_network
        .send(note.clone(), Recipient::All)
        .expect("broadcasting works");
    for _ in 1..2_usize {
        match data.network.send_message.1.lock().next().await {
            Some((_, protocol, message)) => {
                assert_eq!(protocol, PROTOCOL_NAME);
                match InternalMessage::<MockData>::decode_all(message.as_slice()) {
                    Ok(InternalMessage::Data(session_id, data)) => {
                        assert_eq!(session_id, cur_session_id);
                        assert_eq!(data, note);
                    }
                    _ => panic!("Expected a properly encoded message"),
                }
            }
            _ => panic!("Expecting a message"),
        }
    }
    data.complete().await;
}
