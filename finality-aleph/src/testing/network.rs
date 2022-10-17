use std::{
    collections::{HashMap, HashSet},
    iter::FromIterator,
    time::Duration,
};

use aleph_bft::Recipient;
use codec::{Decode, Encode};
use futures::channel::oneshot;
use sc_service::TaskManager;
use tokio::{runtime::Handle, task::JoinHandle, time::timeout};

use crate::{
    crypto::{AuthorityPen, AuthorityVerifier},
    network::{
        mock::{
            crypto_basics, MockData, MockEvent, MockMultiaddress, MockNetwork, MockNetworkIdentity,
            MockPeerId,
        },
        setup_io,
        testing::{Authentication, DataInSession, DiscoveryMessage, NetworkData, SessionHandler},
        ConnectionManager, ConnectionManagerConfig, DataNetwork, NetworkIdentity, Protocol,
        Service as NetworkService, SessionManager,
    },
    testing::mocks::validator_network::MockNetwork as MockValidatorNetwork,
    MillisecsPerBlock, NodeIndex, SessionId, SessionPeriod,
};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const SESSION_PERIOD: SessionPeriod = SessionPeriod(10);
const MILLISECS_PER_BLOCK: MillisecsPerBlock = MillisecsPerBlock(1000);
const NODES_N: usize = 3;

#[derive(Clone)]
struct Authority {
    pen: AuthorityPen,
    addresses: Vec<MockMultiaddress>,
    peer_id: MockPeerId,
}

impl Authority {
    fn pen(&self) -> AuthorityPen {
        self.pen.clone()
    }

    fn addresses(&self) -> Vec<MockMultiaddress> {
        self.addresses.clone()
    }

    fn peer_id(&self) -> MockPeerId {
        self.peer_id
    }
}

impl NetworkIdentity for Authority {
    type PeerId = MockPeerId;
    type Multiaddress = MockMultiaddress;

    fn identity(&self) -> (Vec<Self::Multiaddress>, Self::PeerId) {
        (self.addresses.clone(), self.peer_id)
    }
}

type MockNetworkData = NetworkData<MockData, MockMultiaddress>;

struct TestData {
    pub authorities: Vec<Authority>,
    pub authority_verifier: AuthorityVerifier,
    pub session_manager: SessionManager<MockData>,
    pub network: MockNetwork,
    pub _validator_network: MockValidatorNetwork<DataInSession<MockData>>,
    network_manager_exit_tx: oneshot::Sender<()>,
    legacy_network_manager_exit_tx: oneshot::Sender<()>,
    network_service_exit_tx: oneshot::Sender<()>,
    network_manager_handle: JoinHandle<()>,
    legacy_network_manager_handle: JoinHandle<()>,
    network_service_handle: JoinHandle<()>,
    // `TaskManager` can't be dropped for `SpawnTaskHandle` to work
    _task_manager: TaskManager,
}

async fn prepare_one_session_test_data() -> TestData {
    let task_manager = TaskManager::new(Handle::current(), None).unwrap();
    let (authority_pens, authority_verifier) = crypto_basics(NODES_N).await;
    let authorities: Vec<_> = authority_pens
        .into_iter()
        .map(|(_, p)| {
            let identity = MockNetworkIdentity::new().identity();
            Authority {
                pen: p,
                addresses: identity.0,
                peer_id: identity.1,
            }
        })
        .collect();

    // Prepare Network
    let (event_stream_tx, event_stream_rx) = oneshot::channel();
    let (network_manager_exit_tx, network_manager_exit_rx) = oneshot::channel();
    let (legacy_network_manager_exit_tx, legacy_network_manager_exit_rx) = oneshot::channel();
    let (network_service_exit_tx, network_service_exit_rx) = oneshot::channel();
    let network = MockNetwork::new(event_stream_tx);
    let validator_network = MockValidatorNetwork::new("address").await;

    let (connection_io, network_io, session_io) = setup_io();
    let (legacy_connection_io, legacy_network_io, legacy_session_io) = setup_io();

    let connection_manager = ConnectionManager::new(
        validator_network.clone(),
        ConnectionManagerConfig::with_session_period(&SESSION_PERIOD, &MILLISECS_PER_BLOCK),
    );

    let legacy_connection_manager = ConnectionManager::<Authority, MockData>::new(
        authorities[0].clone(),
        ConnectionManagerConfig::with_session_period(&SESSION_PERIOD, &MILLISECS_PER_BLOCK),
    );

    let session_manager = SessionManager::new(session_io, legacy_session_io);

    let network_service = NetworkService::new(
        network.clone(),
        validator_network.clone(),
        task_manager.spawn_handle(),
        network_io,
        legacy_network_io,
    );

    let network_manager_task = async move {
        tokio::select! {
            _ = connection_io
            .run(connection_manager) => { },
            _ = network_manager_exit_rx => { },
        };
    };

    let legacy_network_manager_task = async move {
        tokio::select! {
            _ = legacy_connection_io
            .run(legacy_connection_manager) => { },
            _ = legacy_network_manager_exit_rx => { },
        };
    };

    let network_service_task = async move {
        tokio::select! {
            _ = network_service.run() => { },
            _ = network_service_exit_rx => { },
        };
    };
    let network_manager_handle = tokio::spawn(network_manager_task);
    let legacy_network_manager_handle = tokio::spawn(legacy_network_manager_task);
    let network_service_handle = tokio::spawn(network_service_task);

    event_stream_rx.await.unwrap();

    TestData {
        authorities,
        authority_verifier,
        session_manager,
        network,
        _validator_network: validator_network,
        network_manager_exit_tx,
        legacy_network_manager_exit_tx,
        network_service_exit_tx,
        network_manager_handle,
        legacy_network_manager_handle,
        network_service_handle,
        _task_manager: task_manager,
    }
}

impl TestData {
    fn connect_identity_to_network(&mut self, peer_id: MockPeerId, protocol: Protocol) {
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

    async fn check_sends_add_reserved_node(&mut self) {
        let mut reserved_addresses = HashSet::new();
        for _ in self.authorities.iter().skip(1) {
            let (addresses, protocol) = timeout(DEFAULT_TIMEOUT, self.network.add_reserved.next())
                .await
                .ok()
                .flatten()
                .expect("Should add reserved nodes");
            assert_eq!(protocol, Protocol::Validator);
            reserved_addresses.extend(addresses.into_iter());
        }

        let mut expected_addresses = HashSet::new();
        for authority in self.authorities.iter().skip(1) {
            expected_addresses.extend(authority.addresses());
        }

        assert_eq!(reserved_addresses, expected_addresses);
    }

    async fn check_sends_authentication(
        &mut self,
        authentication: Authentication<MockMultiaddress>,
    ) {
        let mut sent_auth = HashMap::new();
        while sent_auth.len() < NODES_N - 1 {
            if let Some((
                MockNetworkData::Meta(DiscoveryMessage::Authentication(auth_data)),
                peer_id,
                _,
            )) = timeout(
                DEFAULT_TIMEOUT,
                self.next_sent_authentication(Protocol::Generic),
            )
            .await
            .expect("Should send authentication")
            {
                sent_auth.insert(peer_id, auth_data);
            }
        }

        let mut expected_auth = HashMap::new();
        for authority in self.authorities.iter().skip(1) {
            expected_auth.insert(authority.peer_id(), authentication.clone());
        }

        assert_eq!(sent_auth, expected_auth);
    }

    async fn connect_session_authorities(&mut self, session_id: u32) {
        for (index, authority) in self.authorities.clone().into_iter().enumerate().skip(1) {
            let handler = self.get_session_handler(index, session_id).await;

            self.connect_identity_to_network(authority.peer_id(), Protocol::Generic);
            self.connect_identity_to_network(authority.peer_id(), Protocol::Validator);

            self.network.emit_event(MockEvent::Messages(vec![(
                Protocol::Generic,
                MockNetworkData::Meta(DiscoveryMessage::AuthenticationBroadcast(
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
        self.check_sends_add_reserved_node().await;
        self.check_sends_authentication(
            self.get_session_handler(0, session_id)
                .await
                .authentication()
                .unwrap(),
        )
        .await;

        data_network
    }

    fn emit_notifications_received(&mut self, messages: Vec<MockNetworkData>) {
        self.network.emit_event(MockEvent::Messages(
            messages
                .iter()
                .map(|m| (Protocol::Generic, m.encode().into()))
                .collect(),
        ));
    }

    async fn next_sent(
        &mut self,
        p: Protocol,
    ) -> Option<(
        NetworkData<MockData, MockMultiaddress>,
        MockPeerId,
        Protocol,
    )> {
        loop {
            match self.network.send_message.next().await {
                Some((data, peer_id, protocol)) => {
                    if protocol == p {
                        return Some((
                            NetworkData::<MockData, MockMultiaddress>::decode(&mut data.as_slice())
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

    async fn next_sent_authentication_broadcast(
        &mut self,
        p: Protocol,
    ) -> Option<(
        NetworkData<MockData, MockMultiaddress>,
        MockPeerId,
        Protocol,
    )> {
        loop {
            match self.next_sent(p).await {
                Some((
                    MockNetworkData::Meta(DiscoveryMessage::AuthenticationBroadcast(auth_data)),
                    peer_id,
                    protocol,
                )) => {
                    return Some((
                        MockNetworkData::Meta(DiscoveryMessage::AuthenticationBroadcast(auth_data)),
                        peer_id,
                        protocol,
                    ));
                }
                None => return None,
                _ => {}
            }
        }
    }

    async fn next_sent_authentication(
        &mut self,
        p: Protocol,
    ) -> Option<(
        NetworkData<MockData, MockMultiaddress>,
        MockPeerId,
        Protocol,
    )> {
        loop {
            match self.next_sent(p).await {
                Some((
                    MockNetworkData::Meta(DiscoveryMessage::Authentication(auth_data)),
                    peer_id,
                    protocol,
                )) => {
                    return Some((
                        MockNetworkData::Meta(DiscoveryMessage::Authentication(auth_data)),
                        peer_id,
                        protocol,
                    ));
                }
                None => return None,
                _ => {}
            }
        }
    }

    async fn next_sent_data_message(
        &mut self,
        p: Protocol,
    ) -> Option<(
        NetworkData<MockData, MockMultiaddress>,
        MockPeerId,
        Protocol,
    )> {
        loop {
            match self.next_sent(p).await {
                Some((MockNetworkData::Data(data, session_id), peer_id, protocol)) => {
                    return Some((MockNetworkData::Data(data, session_id), peer_id, protocol))
                }
                None => return None,
                _ => {}
            }
        }
    }

    async fn cleanup(self) {
        self.network_manager_exit_tx.send(()).unwrap();
        self.legacy_network_manager_exit_tx.send(()).unwrap();
        self.network_service_exit_tx.send(()).unwrap();
        self.network_manager_handle.await.unwrap();
        self.legacy_network_manager_handle.await.unwrap();
        self.network_service_handle.await.unwrap();
        while let Some((data, peer_id, protocol)) = self.network.send_message.try_next().await {
            if protocol == Protocol::Validator {
                if let Ok(MockNetworkData::Data(data, session_id)) =
                    MockNetworkData::decode(&mut data.as_slice())
                {
                    panic!("No Data messages to validators should be sent during cleanup. All data messages should be handled before.\
                         Got: {:?} in {:?} to {:?} with protocol {:?}", data, session_id, peer_id, protocol);
                }
            }
        }
        self.network.close_channels().await;
    }
}

#[tokio::test]
async fn test_sends_discovery_message() {
    let session_id = 43;
    let mut test_data = prepare_one_session_test_data().await;
    let connected_peer_id = test_data.authorities[1].peer_id();
    test_data.connect_identity_to_network(connected_peer_id, Protocol::Generic);
    let mut data_network = test_data.start_validator_session(0, session_id).await;
    let handler = test_data.get_session_handler(0, session_id).await;

    let mut sent_legacy_authentications = 0;
    for _ in 0..10 {
        match timeout(
            DEFAULT_TIMEOUT,
            test_data.next_sent_authentication_broadcast(Protocol::Generic),
        )
        .await
        .ok()
        .flatten()
        {
            Some((
                MockNetworkData::Meta(DiscoveryMessage::AuthenticationBroadcast(auth_data)),
                peer_id,
                _,
            )) => {
                assert_eq!(peer_id, connected_peer_id);
                assert_eq!(auth_data, handler.authentication().unwrap());
                sent_legacy_authentications += 1;
            }
            Some(_) => {}
            None => panic!("Should broadcast authentication"),
        }
    }

    if sent_legacy_authentications < 3 {
        panic!("Should broadcast legacy authentications")
    }
    test_data.cleanup().await;
    assert_eq!(
        timeout(DEFAULT_TIMEOUT, data_network.next()).await,
        Ok(None)
    );
}

#[tokio::test]
async fn test_sends_authentication_on_receiving_broadcast() {
    let session_id = 43;
    let mut test_data = prepare_one_session_test_data().await;
    let mut data_network = test_data.start_validator_session(0, session_id).await;
    let handler = test_data.get_session_handler(0, session_id).await;
    let sending_peer_handler = test_data.get_session_handler(1, session_id).await;
    let sending_peer = test_data.authorities[1].clone();
    test_data.connect_identity_to_network(sending_peer.peer_id(), Protocol::Generic);

    test_data.network.emit_event(MockEvent::Messages(vec![(
        Protocol::Generic,
        MockNetworkData::Meta(DiscoveryMessage::AuthenticationBroadcast(
            sending_peer_handler.authentication().unwrap(),
        ))
        .encode()
        .into(),
    )]));

    assert_eq!(
        timeout(DEFAULT_TIMEOUT, test_data.network.add_reserved.next())
            .await
            .ok()
            .flatten()
            .expect("Should add reserved nodes"),
        (
            sending_peer.addresses().into_iter().collect(),
            Protocol::Validator
        ),
    );

    if let Some((MockNetworkData::Meta(DiscoveryMessage::Authentication(auth_data)), peer_id, _)) =
        timeout(
            DEFAULT_TIMEOUT,
            test_data.next_sent_authentication(Protocol::Generic),
        )
        .await
        .expect("Should send authentication")
    {
        assert_eq!(peer_id, sending_peer.peer_id());
        assert_eq!(auth_data, handler.authentication().unwrap());
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
        test_data.connect_identity_to_network(authority.peer_id(), Protocol::Generic);
    }

    test_data.network.emit_event(MockEvent::Messages(vec![(
        Protocol::Generic,
        MockNetworkData::Meta(DiscoveryMessage::AuthenticationBroadcast(
            sending_peer_handler.authentication().unwrap(),
        ))
        .encode()
        .into(),
    )]));

    assert_eq!(
        timeout(DEFAULT_TIMEOUT, test_data.network.add_reserved.next())
            .await
            .ok()
            .flatten()
            .expect("Should add reserved nodes"),
        (
            sending_peer.addresses().into_iter().collect(),
            Protocol::Validator
        ),
    );

    let mut expected_authentication = HashMap::new();
    for authority in test_data.authorities.iter().skip(1) {
        expected_authentication.insert(
            authority.peer_id(),
            sending_peer_handler.authentication().unwrap(),
        );
    }

    let mut sent_authentication = HashMap::new();
    while sent_authentication.len() < NODES_N - 1 {
        if let Some((
            MockNetworkData::Meta(DiscoveryMessage::AuthenticationBroadcast(auth_data)),
            peer_id,
            _,
        )) = timeout(
            DEFAULT_TIMEOUT,
            test_data.next_sent_authentication_broadcast(Protocol::Generic),
        )
        .await
        .expect("Should send Authentication Broadcast")
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
    test_data.emit_notifications_received(vec![MockNetworkData::Data(
        data.clone(),
        SessionId(session_id),
    )]);
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
    test_data.check_sends_add_reserved_node().await;
    test_data
        .check_sends_authentication(
            test_data
                .get_session_handler(0, session_id)
                .await
                .authentication()
                .unwrap(),
        )
        .await;
    let mut data_network = test_data.start_validator_session(0, session_id).await;

    let data = vec![1, 2, 3];
    test_data.emit_notifications_received(vec![MockNetworkData::Data(
        data.clone(),
        SessionId(session_id),
    )]);
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
    assert_eq!(
        timeout(DEFAULT_TIMEOUT, test_data.network.remove_reserved.next())
            .await
            .ok()
            .flatten(),
        Some((
            HashSet::from_iter(test_data.authorities.iter().skip(1).map(|a| a.peer_id())),
            Protocol::Validator
        ))
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
    test_data.emit_notifications_received(vec![
        MockNetworkData::Data(data_1_1.clone(), SessionId(session_id_1)),
        MockNetworkData::Data(data_2_1.clone(), SessionId(session_id_2)),
    ]);
    test_data.emit_notifications_received(vec![
        MockNetworkData::Data(data_2_2.clone(), SessionId(session_id_2)),
        MockNetworkData::Data(data_1_2.clone(), SessionId(session_id_1)),
    ]);

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
        if let Some((MockNetworkData::Data(data, session_id), peer_id, _)) = timeout(
            DEFAULT_TIMEOUT,
            test_data.next_sent_data_message(Protocol::Validator),
        )
        .await
        .expect("Should send data")
        {
            println!("{:?} {:?}", data, peer_id);
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
        if let Some((MockNetworkData::Data(data, session_id), peer_id, _)) = timeout(
            DEFAULT_TIMEOUT,
            test_data.next_sent_data_message(Protocol::Validator),
        )
        .await
        .expect("Should send data")
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
