use std::{
    collections::{HashMap, HashSet},
    iter::FromIterator,
    time::Duration,
};

use codec::{Decode, Encode};
use futures::channel::oneshot;
use sc_service::TaskManager;
use tokio::{runtime::Handle, task::JoinHandle, time::timeout};

use crate::{
    crypto::{AuthorityPen, AuthorityVerifier},
    network::{
        mock::{crypto_basics, MockData, MockEvent, MockNetwork},
        setup_io,
        testing::{DataInSession, DiscoveryMessage, SessionHandler, VersionedAuthentication},
        ConnectionManager, ConnectionManagerConfig, DataNetwork, NetworkIdentity, Protocol,
        Service as NetworkService, SessionManager,
    },
    testing::mocks::validator_network::{
        random_identity_with_address, MockMultiaddress, MockNetwork as MockValidatorNetwork,
    },
    validator_network::mock::{key, MockPublicKey},
    MillisecsPerBlock, NodeIndex, Recipient, SessionId, SessionPeriod,
};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const SESSION_PERIOD: SessionPeriod = SessionPeriod(10);
const MILLISECS_PER_BLOCK: MillisecsPerBlock = MillisecsPerBlock(1000);
const NODES_N: usize = 3;

#[derive(Clone)]
struct Authority {
    pen: AuthorityPen,
    addresses: Vec<MockMultiaddress>,
    peer_id: MockPublicKey,
    auth_peer_id: MockPublicKey,
}

impl Authority {
    fn pen(&self) -> AuthorityPen {
        self.pen.clone()
    }

    fn addresses(&self) -> Vec<MockMultiaddress> {
        self.addresses.clone()
    }

    fn peer_id(&self) -> MockPublicKey {
        self.peer_id.clone()
    }

    fn auth_peer_id(&self) -> MockPublicKey {
        self.auth_peer_id.clone()
    }
}

impl NetworkIdentity for Authority {
    type PeerId = MockPublicKey;
    type Multiaddress = MockMultiaddress;

    fn identity(&self) -> (Vec<Self::Multiaddress>, Self::PeerId) {
        (self.addresses.clone(), self.peer_id.clone())
    }
}

struct TestData {
    pub authorities: Vec<Authority>,
    pub authority_verifier: AuthorityVerifier,
    pub session_manager: SessionManager<MockData>,
    pub network: MockNetwork,
    pub validator_network: MockValidatorNetwork<DataInSession<MockData>>,
    network_manager_exit_tx: oneshot::Sender<()>,
    network_service_exit_tx: oneshot::Sender<()>,
    network_manager_handle: JoinHandle<()>,
    network_service_handle: JoinHandle<()>,
    // `TaskManager` can't be dropped for `SpawnTaskHandle` to work
    _task_manager: TaskManager,
}

async fn prepare_one_session_test_data() -> TestData {
    let task_manager = TaskManager::new(Handle::current(), None).unwrap();
    let (authority_pens, authority_verifier) = crypto_basics(NODES_N).await;
    let mut authorities = Vec::new();
    for (index, p) in authority_pens {
        let identity = random_identity_with_address(index.0.to_string());
        let auth_peer_id = key().0;
        authorities.push(Authority {
            pen: p,
            addresses: identity.0,
            peer_id: identity.1,
            auth_peer_id,
        });
    }

    // Prepare Network
    let (event_stream_tx, event_stream_rx) = oneshot::channel();
    let (network_manager_exit_tx, network_manager_exit_rx) = oneshot::channel();
    let (network_service_exit_tx, network_service_exit_rx) = oneshot::channel();
    let network = MockNetwork::new(event_stream_tx);
    let validator_network =
        MockValidatorNetwork::from(authorities[0].addresses(), authorities[0].peer_id());

    let (connection_io, network_io, session_io) = setup_io(validator_network.clone());

    let connection_manager = ConnectionManager::new(
        validator_network.clone(),
        ConnectionManagerConfig::with_session_period(&SESSION_PERIOD, &MILLISECS_PER_BLOCK),
    );

    let session_manager = SessionManager::new(session_io);

    let network_service =
        NetworkService::new(network.clone(), task_manager.spawn_handle(), network_io);

    let network_manager_task = async move {
        tokio::select! {
            _ = connection_io
            .run(connection_manager) => { },
            _ = network_manager_exit_rx => { },
        };
    };

    let network_service_task = async move {
        tokio::select! {
            _ = network_service.run() => { },
            _ = network_service_exit_rx => { },
        };
    };
    let network_manager_handle = tokio::spawn(network_manager_task);
    let network_service_handle = tokio::spawn(network_service_task);

    event_stream_rx.await.unwrap();

    TestData {
        authorities,
        authority_verifier,
        session_manager,
        network,
        validator_network,
        network_manager_exit_tx,
        network_service_exit_tx,
        network_manager_handle,
        network_service_handle,
        _task_manager: task_manager,
    }
}

impl TestData {
    fn connect_identity_to_network(&mut self, peer_id: MockPublicKey, protocol: Protocol) {
        self.network
            .emit_event(MockEvent::StreamOpened(peer_id, protocol));
    }

    async fn start_validator_session(
        &self,
        node_id: usize,
        session_id: u32,
    ) -> impl DataNetwork<MockData> {
        self.session_manager
            .start_validator_session(
                SessionId(session_id),
                self.authority_verifier.clone(),
                NodeIndex(node_id),
                self.authorities[node_id].pen(),
            )
            .await
            .expect("Failed to start validator session!")
    }

    fn early_start_validator_session(&self, node_id: usize, session_id: u32) {
        self.session_manager
            .early_start_validator_session(
                SessionId(session_id),
                self.authority_verifier.clone(),
                NodeIndex(node_id),
                self.authorities[node_id].pen(),
            )
            .expect("Failed to start validator session!");
    }

    async fn get_session_handler(
        &self,
        node_id: usize,
        session_id: u32,
    ) -> SessionHandler<MockMultiaddress> {
        SessionHandler::new(
            Some((NodeIndex(node_id), self.authorities[node_id].pen())),
            self.authority_verifier.clone(),
            SessionId(session_id),
            self.authorities[node_id].addresses().to_vec(),
        )
        .await
        .unwrap()
    }

    async fn check_add_connection(&mut self) {
        let mut reserved_addresses = HashSet::new();
        for _ in self.authorities.iter().skip(1) {
            let (_, addresses) = self
                .validator_network
                .add_connection
                .next()
                .await
                .expect("Should add reserved nodes");
            reserved_addresses.extend(addresses.into_iter());
        }

        let mut expected_addresses = HashSet::new();
        for authority in self.authorities.iter().skip(1) {
            expected_addresses.extend(authority.addresses());
        }

        assert_eq!(reserved_addresses, expected_addresses);
    }

    async fn connect_session_authorities(&mut self, session_id: u32) {
        for (index, authority) in self.authorities.clone().into_iter().enumerate().skip(1) {
            let handler = self.get_session_handler(index, session_id).await;

            self.connect_identity_to_network(authority.auth_peer_id(), Protocol::Authentication);

            self.network.emit_event(MockEvent::Messages(vec![(
                Protocol::Authentication,
                VersionedAuthentication::V1(DiscoveryMessage::AuthenticationBroadcast(
                    handler.authentication().unwrap(),
                ))
                .encode()
                .into(),
            )]));
        }
    }

    async fn start_session(&mut self, session_id: u32) -> impl DataNetwork<MockData> {
        let data_network = self.start_validator_session(0, session_id).await;
        self.connect_session_authorities(session_id).await;
        self.check_add_connection().await;

        data_network
    }

    async fn next_sent_auth(
        &mut self,
    ) -> Option<(
        VersionedAuthentication<MockMultiaddress>,
        MockPublicKey,
        Protocol,
    )> {
        loop {
            match self.network.send_message.next().await {
                Some((data, peer_id, protocol)) => {
                    if protocol == Protocol::Authentication {
                        return Some((
                            VersionedAuthentication::<MockMultiaddress>::decode(
                                &mut data.as_slice(),
                            )
                            .expect("should decode"),
                            peer_id,
                            protocol,
                        ));
                    };
                }
                None => return None,
            }
        }
    }

    async fn cleanup(self) {
        self.network_manager_exit_tx.send(()).unwrap();
        self.network_service_exit_tx.send(()).unwrap();
        self.network_manager_handle.await.unwrap();
        self.network_service_handle.await.unwrap();
        while self.network.send_message.try_next().await.is_some() {}
        self.network.close_channels().await;
        self.validator_network.close_channels().await;
    }
}

#[tokio::test]
async fn test_sends_discovery_message() {
    let session_id = 43;
    let mut test_data = prepare_one_session_test_data().await;
    let connected_peer_id = test_data.authorities[1].auth_peer_id();
    test_data.connect_identity_to_network(connected_peer_id.clone(), Protocol::Authentication);
    let mut data_network = test_data.start_validator_session(0, session_id).await;
    let handler = test_data.get_session_handler(0, session_id).await;

    for _ in 0..4 {
        match test_data.next_sent_auth().await {
            Some((
                VersionedAuthentication::V1(DiscoveryMessage::AuthenticationBroadcast(auth_data)),
                peer_id,
                _,
            )) => {
                assert_eq!(peer_id, connected_peer_id);
                assert_eq!(auth_data, handler.authentication().unwrap());
            }
            None => panic!("Not sending authentications"),
            _ => panic!("Should broadcast own authentication, nothing else"),
        }
    }

    test_data.cleanup().await;
    assert_eq!(
        timeout(DEFAULT_TIMEOUT, data_network.next()).await,
        Ok(None)
    );
}

#[tokio::test]
async fn test_forwards_authentication_broadcast() {
    let session_id = 43;
    let mut test_data = prepare_one_session_test_data().await;
    let mut data_network = test_data.start_validator_session(0, session_id).await;
    let handler = test_data.get_session_handler(0, session_id).await;
    let sending_peer = test_data.authorities[1].clone();
    let sending_peer_handler = test_data.get_session_handler(1, session_id).await;

    for authority in test_data.authorities.clone().iter().skip(1) {
        test_data.connect_identity_to_network(authority.auth_peer_id(), Protocol::Authentication);
    }

    test_data.network.emit_event(MockEvent::Messages(vec![(
        Protocol::Authentication,
        VersionedAuthentication::V1(DiscoveryMessage::AuthenticationBroadcast(
            sending_peer_handler.authentication().unwrap(),
        ))
        .encode()
        .into(),
    )]));

    assert_eq!(
        test_data
            .validator_network
            .add_connection
            .next()
            .await
            .expect("Should add reserved nodes"),
        (sending_peer.peer_id(), sending_peer.addresses()),
    );

    let mut expected_authentication = HashMap::new();
    for authority in test_data.authorities.iter().skip(1) {
        expected_authentication.insert(
            authority.auth_peer_id(),
            sending_peer_handler.authentication().unwrap(),
        );
    }

    let mut sent_authentication = HashMap::new();
    while sent_authentication.len() < NODES_N - 1 {
        if let Some((
            VersionedAuthentication::V1(DiscoveryMessage::AuthenticationBroadcast(auth_data)),
            peer_id,
            _,
        )) = test_data.next_sent_auth().await
        {
            if auth_data != handler.authentication().unwrap() {
                sent_authentication.insert(peer_id, auth_data);
            }
        }
    }

    assert_eq!(sent_authentication, expected_authentication);

    test_data.cleanup().await;
    assert_eq!(
        timeout(DEFAULT_TIMEOUT, data_network.next()).await,
        Ok(None)
    );
}

#[tokio::test]
async fn test_connects_to_others() {
    let session_id = 43;
    let mut test_data = prepare_one_session_test_data().await;
    let mut data_network = test_data.start_session(session_id).await;

    let data = vec![1, 2, 3];
    test_data.validator_network.next.send(DataInSession {
        data: data.clone(),
        session_id: SessionId(session_id),
    });

    assert_eq!(
        timeout(DEFAULT_TIMEOUT, data_network.next()).await,
        Ok(Some(data))
    );

    test_data.cleanup().await;
    assert_eq!(data_network.next().await, None);
}

#[tokio::test]
async fn test_connects_to_others_early_validator() {
    let session_id = 43;
    let mut test_data = prepare_one_session_test_data().await;
    test_data.early_start_validator_session(0, session_id);
    test_data.connect_session_authorities(session_id).await;
    test_data.check_add_connection().await;

    let mut data_network = test_data.start_validator_session(0, session_id).await;

    let data = vec![1, 2, 3];
    test_data.validator_network.next.send(DataInSession {
        data: data.clone(),
        session_id: SessionId(session_id),
    });
    assert_eq!(
        timeout(DEFAULT_TIMEOUT, data_network.next()).await,
        Ok(Some(data.clone()))
    );

    test_data.cleanup().await;
    assert_eq!(data_network.next().await, None);
}

#[tokio::test]
async fn test_stops_session() {
    let session_id = 43;
    let mut test_data = prepare_one_session_test_data().await;
    let mut data_network = test_data.start_session(session_id).await;

    test_data
        .session_manager
        .stop_session(SessionId(session_id))
        .unwrap();

    let removed = HashSet::<_>::from_iter(
        test_data
            .validator_network
            .remove_connection
            .take(NODES_N - 1)
            .await
            .into_iter(),
    );
    assert_eq!(
        removed,
        HashSet::from_iter(test_data.authorities.iter().skip(1).map(|a| a.peer_id())),
    );

    // This assert should be before cleanup. We want to check whether `session_manager.stop_session(...)`
    // drops the sender. After cleanup all network tasks end and senders will be dropped.
    // If assert was after cleanup we wouldn't know whether data_network receiver is droopped
    // because of `session_manager.stop_session(...)` or because of cleanup.
    assert_eq!(
        timeout(DEFAULT_TIMEOUT, data_network.next()).await,
        Ok(None)
    );
    test_data.cleanup().await;
}

#[tokio::test]
async fn test_receives_data_in_correct_session() {
    let session_id_1 = 42;
    let session_id_2 = 43;
    let mut test_data = prepare_one_session_test_data().await;
    let mut data_network_1 = test_data.start_session(session_id_1).await;

    let mut data_network_2 = test_data.start_session(session_id_2).await;

    let data_1_1 = vec![1, 2, 3];
    let data_1_2 = vec![4, 5, 6];
    let data_2_1 = vec![7, 8, 9];
    let data_2_2 = vec![10, 11, 12];
    test_data.validator_network.next.send(DataInSession {
        data: data_1_1.clone(),
        session_id: SessionId(session_id_1),
    });
    test_data.validator_network.next.send(DataInSession {
        data: data_2_1.clone(),
        session_id: SessionId(session_id_2),
    });

    test_data.validator_network.next.send(DataInSession {
        data: data_2_2.clone(),
        session_id: SessionId(session_id_2),
    });
    test_data.validator_network.next.send(DataInSession {
        data: data_1_2.clone(),
        session_id: SessionId(session_id_1),
    });

    assert_eq!(
        timeout(DEFAULT_TIMEOUT, data_network_1.next()).await,
        Ok(Some(data_1_1))
    );
    assert_eq!(
        timeout(DEFAULT_TIMEOUT, data_network_1.next()).await,
        Ok(Some(data_1_2))
    );
    assert_eq!(
        timeout(DEFAULT_TIMEOUT, data_network_2.next()).await,
        Ok(Some(data_2_1))
    );
    assert_eq!(
        timeout(DEFAULT_TIMEOUT, data_network_2.next()).await,
        Ok(Some(data_2_2))
    );

    test_data.cleanup().await;

    assert_eq!(
        timeout(DEFAULT_TIMEOUT, data_network_1.next()).await,
        Ok(None)
    );
    assert_eq!(
        timeout(DEFAULT_TIMEOUT, data_network_2.next()).await,
        Ok(None)
    );
}

#[tokio::test]
async fn test_sends_data_to_correct_session() {
    let session_id_1 = 42;
    let session_id_2 = 43;
    let mut test_data = prepare_one_session_test_data().await;
    let mut data_network_1 = test_data.start_session(session_id_1).await;
    let mut data_network_2 = test_data.start_session(session_id_2).await;

    let mut expected_data = HashSet::new();
    for node_id in 1..NODES_N {
        let data_1 = vec![2 * node_id as u8 - 1];
        let data_2 = vec![2 * node_id as u8];

        expected_data.insert((
            data_1.clone(),
            SessionId(session_id_1),
            test_data.authorities[node_id].peer_id(),
        ));
        data_network_1
            .send(data_1, Recipient::Node(NodeIndex(node_id)))
            .expect("Should send");

        expected_data.insert((
            data_2.clone(),
            SessionId(session_id_2),
            test_data.authorities[node_id].peer_id(),
        ));
        data_network_2
            .send(data_2, Recipient::Node(NodeIndex(node_id)))
            .expect("Should send");
    }

    let mut sent_data = HashSet::new();
    while sent_data.len() < 2 * (NODES_N - 1) {
        if let Some((DataInSession { data, session_id }, peer_id)) =
            test_data.validator_network.send.next().await
        {
            sent_data.insert((data, session_id, peer_id));
        }
    }

    assert_eq!(sent_data, expected_data);
    test_data.cleanup().await;

    assert_eq!(
        timeout(DEFAULT_TIMEOUT, data_network_1.next()).await,
        Ok(None)
    );
    assert_eq!(
        timeout(DEFAULT_TIMEOUT, data_network_2.next()).await,
        Ok(None)
    );
}

#[tokio::test]
async fn test_broadcasts_data_to_correct_session() {
    let session_id_1 = 42;
    let session_id_2 = 43;
    let mut test_data = prepare_one_session_test_data().await;
    let mut data_network_1 = test_data.start_session(session_id_1).await;
    let mut data_network_2 = test_data.start_session(session_id_2).await;

    let data_1 = vec![1, 2, 3];
    let data_2 = vec![4, 5, 6];
    data_network_1
        .send(data_1.clone(), Recipient::Everyone)
        .expect("Should send");
    data_network_2
        .send(data_2.clone(), Recipient::Everyone)
        .expect("Should send");

    let mut expected_data = HashSet::new();
    for authority in test_data.authorities.iter().skip(1) {
        expected_data.insert((data_1.clone(), SessionId(session_id_1), authority.peer_id()));
        expected_data.insert((data_2.clone(), SessionId(session_id_2), authority.peer_id()));
    }

    let mut sent_data = HashSet::new();
    while sent_data.len() < 2 * (NODES_N - 1) {
        if let Some((DataInSession { data, session_id }, peer_id)) =
            test_data.validator_network.send.next().await
        {
            sent_data.insert((data, session_id, peer_id));
        }
    }

    assert_eq!(sent_data, expected_data);

    test_data.cleanup().await;

    assert_eq!(
        timeout(DEFAULT_TIMEOUT, data_network_1.next()).await,
        Ok(None)
    );
    assert_eq!(
        timeout(DEFAULT_TIMEOUT, data_network_2.next()).await,
        Ok(None)
    );
}
