use super::*;
use crate::KEY_TYPE;
use futures::{
    channel::{mpsc, oneshot},
    SinkExt,
};
use sc_network::{Event, ObservedRole, PeerId, ReputationChange};
use sp_keystore::{testing::KeyStore, CryptoStore};
use sp_runtime::traits::Block as BlockT;
use std::{collections::HashSet, time::Duration};
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
}

impl<B: BlockT> TestNetwork<B> {
    fn new(tx: oneshot::Sender<()>) -> Self {
        TestNetwork {
            event_sinks: Arc::new(Mutex::new(vec![])),
            oneshot_sender: Arc::new(Mutex::new(Some(tx))),
            report_peer: channel(),
            disconnect_peer: channel(),
            send_message: channel(),
            announce: channel(),
            add_set_reserved: channel(),
            remove_set_reserved: channel(),
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
    id: AuthorityId,
    peer_id: PeerId,
}

async fn generate_authority(s: &str) -> Authority {
    let key_store = Arc::new(KeyStore::new());
    let pk = key_store
        .sr25519_generate_new(KEY_TYPE, Some(s))
        .await
        .unwrap();
    assert_eq!(key_store.keys(KEY_TYPE).await.unwrap().len(), 3);
    let id = AuthorityId::from(pk);
    let peer_id = PeerId::random();
    Authority { id, peer_id }
}

struct TestData {
    network: TestNetwork<Block>,
    _alice: Authority,
    bob: Authority,
    charlie: Authority,
    rush_network: RushNetwork,
    consensus_network_handle: tokio::task::JoinHandle<()>,
}

impl TestData {
    // consumes the test data asserting there are no unread messages in the channels
    // and awaits for the consensus_network task.
    async fn complete(mut self) {
        self.network.close_channels();
        self.rush_network.net_command_tx.close_channel();
        assert!(self.rush_network.net_event_rx.try_next().is_err());
        self.consensus_network_handle.await.unwrap();
    }
}

const PROTOCOL_NAME: &str = "/test/1";

async fn prepare_one_session_test_data() -> TestData {
    let (oneshot_tx, oneshot_rx) = oneshot::channel();
    let network = TestNetwork::<Block>::new(oneshot_tx);
    let consensus_network =
        ConsensusNetwork::<Block, TestNetwork<Block>>::new(network.clone(), PROTOCOL_NAME);

    let _alice = generate_authority("//Alice").await;
    let bob = generate_authority("//Bob").await;
    let charlie = generate_authority("//Charlie").await;

    let authorities: Vec<_> = [&_alice, &bob, &charlie]
        .iter()
        .map(|auth| auth.id.clone())
        .collect();
    let session_id = SessionId(0);

    let rush_network = consensus_network
        .session_manager()
        .start_session(session_id, authorities);
    let consensus_network_handle = tokio::spawn(async move { consensus_network.run().await });

    // wait till consensus_network takes the event_stream
    oneshot_rx.await.unwrap();

    TestData {
        network,
        _alice,
        bob,
        charlie,
        rush_network,
        consensus_network_handle,
    }
}

#[tokio::test]
async fn test_network_event_sync_connnected() {
    let data = prepare_one_session_test_data().await;
    data.network.emit_event(Event::SyncConnected {
        remote: data.bob.peer_id,
    });
    let (peer_id, protocol) = data.network.add_set_reserved.1.lock().next().await.unwrap();
    assert_eq!(peer_id, data.bob.peer_id);
    assert_eq!(protocol, PROTOCOL_NAME);
    data.complete().await;
}
#[tokio::test]
async fn test_network_event_sync_disconnected() {
    let data = prepare_one_session_test_data().await;
    data.network.emit_event(Event::SyncDisconnected {
        remote: data.charlie.peer_id,
    });
    let (peer_id, protocol) = data
        .network
        .remove_set_reserved
        .1
        .lock()
        .next()
        .await
        .unwrap();
    assert_eq!(peer_id, data.charlie.peer_id);
    assert_eq!(protocol, PROTOCOL_NAME);
    data.complete().await;
}

#[tokio::test]
async fn test_network_event_notifications_received() {
    let bytes: Vec<u8> = (0..=255).collect();
    let mut data = prepare_one_session_test_data().await;
    let encoded_message: Vec<u8> =
        <(SessionId, Vec<u8>) as Encode>::encode(&(data.rush_network.session_id, bytes.clone()));
    let messages = vec![(PROTOCOL_NAME.into(), encoded_message.into())];

    data.network.emit_event(Event::NotificationsReceived {
        remote: data.bob.peer_id,
        messages,
    });
    if let Some(NetworkEvent::MessageReceived(message, peer_id_bytes)) =
        data.rush_network.net_event_rx.next().await
    {
        assert_eq!(message, bytes);
        assert_eq!(peer_id_bytes, data.bob.peer_id.to_bytes());
    } else {
        panic!("expected message received network event")
    }
    data.complete().await;
}
#[tokio::test]
async fn test_network_commands() {
    let mut data = prepare_one_session_test_data().await;
    data.network.emit_event(Event::NotificationStreamOpened {
        remote: data.bob.peer_id,
        protocol: PROTOCOL_NAME.into(),
        role: ObservedRole::Authority,
    });
    data.network.emit_event(Event::NotificationStreamOpened {
        remote: data.charlie.peer_id,
        protocol: PROTOCOL_NAME.into(),
        role: ObservedRole::Authority,
    });

    println!("send to peer");

    // SendToPeer
    {
        let fake_message: Vec<u8> = vec![157];
        data.rush_network
            .net_command_tx
            .send(SessionCommand {
                session_id: data.rush_network.session_id,
                command: NetworkCommand::SendToPeer(
                    fake_message.clone(),
                    data.bob.peer_id.to_bytes(),
                ),
            })
            .await
            .unwrap();
        match data.network.send_message.1.lock().next().await {
            Some((peer_id, protocol, message)) => {
                assert_eq!(peer_id, data.bob.peer_id);
                assert_eq!(protocol, PROTOCOL_NAME);
                match <(SessionId, Vec<u8>) as Decode>::decode(&mut message.as_slice()) {
                    Ok((session_id_, message)) => {
                        assert_eq!(session_id_.0, 0);
                        assert_eq!(message, fake_message);
                    }
                    _ => panic!("Expected a properly encoded message"),
                }
            }
            _ => panic!("Expecting a message"),
        }
    }

    println!("send to all");

    // SendToAll
    {
        let fake_message: Vec<u8> = vec![205];
        data.rush_network
            .net_command_tx
            .send(SessionCommand {
                session_id: data.rush_network.session_id,
                command: NetworkCommand::SendToAll(fake_message.clone()),
            })
            .await
            .unwrap();
        let mut peer_ids = HashSet::<PeerId>::new();
        for _ in 0..2_u8 {
            match data.network.send_message.1.lock().next().await {
                Some((peer_id, protocol, message)) => {
                    peer_ids.insert(peer_id);
                    assert_eq!(protocol, PROTOCOL_NAME);
                    match <(SessionId, Vec<u8>)>::decode(&mut message.as_slice()) {
                        Ok((session_id, message)) => {
                            assert_eq!(session_id.0, 0);
                            assert_eq!(message, fake_message);
                        }
                        _ => panic!("Expected a properly encoded message"),
                    }
                }
                _ => panic!("Expected two messages"),
            }
        }
        let expected_peer_ids: HashSet<_> = [data.bob.peer_id, data.charlie.peer_id]
            .iter()
            .cloned()
            .collect();
        assert_eq!(peer_ids, expected_peer_ids);
    }

    // SendToRandPeer
    {
        let fake_message = vec![74];
        data.rush_network
            .net_command_tx
            .send(SessionCommand {
                session_id: data.rush_network.session_id,
                command: NetworkCommand::SendToRandPeer(fake_message.clone()),
            })
            .await
            .unwrap();
        match data.network.send_message.1.lock().next().await {
            Some((peer_id, protocol, message)) => {
                assert!(peer_id == data.bob.peer_id || peer_id == data.charlie.peer_id);
                assert_eq!(protocol, PROTOCOL_NAME);
                match <(SessionId, Vec<u8>)>::decode(&mut message.as_slice()) {
                    Ok((session_id, message)) => {
                        assert_eq!(session_id.0, 0);
                        assert_eq!(message, fake_message);
                    }
                    _ => panic!("Expected a properly encoded message"),
                }
            }
            _ => panic!("Expected a message"),
        }
    }

    // SendToRandPeer after bob disconnects
    {
        println!("{:?}", data.bob.peer_id);
        data.network.emit_event(Event::NotificationStreamClosed {
            remote: data.bob.peer_id,
            protocol: PROTOCOL_NAME.into(),
        });
        let fake_message = vec![180];
        data.rush_network
            .net_command_tx
            .send(SessionCommand {
                session_id: data.rush_network.session_id,
                command: NetworkCommand::SendToRandPeer(fake_message.clone()),
            })
            .await
            .unwrap();
        // wait for a moment to make sure that bob is disconnected.
        tokio::time::delay_for(Duration::from_millis(500)).await;
        match data.network.send_message.1.lock().next().await {
            Some((peer_id, protocol, message)) => {
                assert_eq!(peer_id, data.charlie.peer_id);
                assert_eq!(protocol, PROTOCOL_NAME);
                match <(SessionId, Vec<u8>)>::decode(&mut message.as_slice()) {
                    Ok((session_id, message)) => {
                        assert_eq!(session_id.0, 0);
                        assert_eq!(message, fake_message);
                    }
                    _ => panic!("Expected a properly encoded message"),
                }
            }
            _ => panic!("Expected a message"),
        }
    }

    data.complete().await;
}
