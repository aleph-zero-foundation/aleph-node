use std::{
    collections::{BTreeMap, HashMap, HashSet},
    io::Result as IoResult,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use aleph_primitives::{AuthorityId, KEY_TYPE};
use codec::{Decode, Encode, Output};
use futures::{
    channel::{mpsc, oneshot},
    StreamExt,
};
use log::info;
use rand::{thread_rng, Rng};
use sc_service::{SpawnTaskHandle, TaskManager};
use sp_keystore::{testing::KeyStore, CryptoStore};
use tokio::{
    io::{duplex, AsyncRead, AsyncWrite, DuplexStream, ReadBuf},
    runtime::Handle,
    time::{error::Elapsed, interval, timeout, Duration},
};

use crate::{
    crypto::AuthorityPen,
    network::{mock::Channel, Data, Multiaddress, NetworkIdentity},
    validator_network::{
        mock::random_keys, ConnectionInfo, Dialer as DialerT, Listener as ListenerT, Network,
        PeerAddressInfo, Service, Splittable,
    },
};

pub type MockMultiaddress = (AuthorityId, String);

impl Multiaddress for MockMultiaddress {
    type PeerId = AuthorityId;

    fn get_peer_id(&self) -> Option<Self::PeerId> {
        Some(self.0.clone())
    }

    fn add_matching_peer_id(self, peer_id: Self::PeerId) -> Option<Self> {
        match self.0 == peer_id {
            true => Some(self),
            false => None,
        }
    }
}

#[derive(Clone)]
pub struct MockNetwork<D: Data> {
    pub add_connection: Channel<(AuthorityId, Vec<MockMultiaddress>)>,
    pub remove_connection: Channel<AuthorityId>,
    pub send: Channel<(D, AuthorityId)>,
    pub next: Channel<D>,
    id: AuthorityId,
    addresses: Vec<MockMultiaddress>,
}

#[async_trait::async_trait]
impl<D: Data> Network<AuthorityId, MockMultiaddress, D> for MockNetwork<D> {
    fn add_connection(&mut self, peer: AuthorityId, addresses: Vec<MockMultiaddress>) {
        self.add_connection.send((peer, addresses));
    }

    fn remove_connection(&mut self, peer: AuthorityId) {
        self.remove_connection.send(peer);
    }

    fn send(&self, data: D, recipient: AuthorityId) {
        self.send.send((data, recipient));
    }

    async fn next(&mut self) -> Option<D> {
        self.next.next().await
    }
}

impl<D: Data> NetworkIdentity for MockNetwork<D> {
    type PeerId = AuthorityId;
    type Multiaddress = MockMultiaddress;

    fn identity(&self) -> (Vec<Self::Multiaddress>, Self::PeerId) {
        (self.addresses.clone(), self.id.clone())
    }
}

pub async fn random_authority_id() -> AuthorityId {
    let key_store = Arc::new(KeyStore::new());
    key_store
        .ed25519_generate_new(KEY_TYPE, None)
        .await
        .unwrap()
        .into()
}

pub async fn random_identity(address: String) -> (Vec<MockMultiaddress>, AuthorityId) {
    let id = random_authority_id().await;
    (vec![(id.clone(), address)], id)
}

impl<D: Data> MockNetwork<D> {
    pub async fn new(address: &str) -> Self {
        let id = random_authority_id().await;
        let addresses = vec![(id.clone(), String::from(address))];
        MockNetwork {
            add_connection: Channel::new(),
            remove_connection: Channel::new(),
            send: Channel::new(),
            next: Channel::new(),
            addresses,
            id,
        }
    }

    pub fn from(addresses: Vec<MockMultiaddress>, id: AuthorityId) -> Self {
        MockNetwork {
            add_connection: Channel::new(),
            remove_connection: Channel::new(),
            send: Channel::new(),
            next: Channel::new(),
            addresses,
            id,
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
type Addresses = HashMap<AuthorityId, Vec<Address>>;
type Callers = HashMap<AuthorityId, (MockDialer, MockListener)>;
type Connection = UnreliableSplittable;

const TWICE_MAX_DATA_SIZE: usize = 32 * 1024 * 1024;

#[derive(Clone)]
pub struct MockDialer {
    // used for logging
    own_address: Address,
    channel_connect: mpsc::UnboundedSender<(Address, Address, oneshot::Sender<Connection>)>,
}

#[async_trait::async_trait]
impl DialerT<Address> for MockDialer {
    type Connection = Connection;
    type Error = std::io::Error;

    async fn connect(&mut self, addresses: Vec<Address>) -> Result<Self::Connection, Self::Error> {
        let (tx, rx) = oneshot::channel();
        self.channel_connect
            .unbounded_send((self.own_address, addresses[0], tx))
            .expect("should send");
        Ok(rx.await.expect("should receive"))
    }
}

pub struct MockListener {
    channel_accept: mpsc::UnboundedReceiver<Connection>,
}

#[async_trait::async_trait]
impl ListenerT for MockListener {
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
    pub fn new(ids: Vec<AuthorityId>) -> (Self, Callers, Addresses) {
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
            .map(|(id, u)| (id, vec![u as u32]))
            .collect();
        // create callers for every peer, keep channels for communicating with them
        for id in ids.into_iter() {
            let (tx_listener, rx_listener) = mpsc::unbounded();
            let dialer = MockDialer {
                own_address: addr.get(&id).expect("should be there")[0],
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
            info!(target: "validator-network", "UnreliableConnectionMaker: waiting for new request...");
            let (dialer_address, listener_address, c) =
                self.dialers.next().await.expect("should receive");
            info!(target: "validator-network", "UnreliableConnectionMaker: received request");
            let (dialer_stream, listener_stream) = Connection::new(
                4096,
                connections_end_after,
                dialer_address,
                listener_address,
            );
            info!(target: "validator-network", "UnreliableConnectionMaker: sending stream");
            c.send(dialer_stream).expect("should send");
            self.listeners[listener_address as usize]
                .unbounded_send(listener_stream)
                .expect("should send");
        }
    }
}

#[derive(Clone)]
struct MockData {
    data: u32,
    filler: Vec<u8>,
    decodes: bool,
}

impl MockData {
    fn new(data: u32, filler_size: usize, decodes: bool) -> MockData {
        MockData {
            data,
            filler: vec![0; filler_size],
            decodes,
        }
    }
}

impl Encode for MockData {
    fn size_hint(&self) -> usize {
        self.data.size_hint() + self.filler.size_hint() + self.decodes.size_hint()
    }

    fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
        // currently this is exactly the default behaviour, but we still
        // need it here to make sure that decode works in the future
        self.data.encode_to(dest);
        self.filler.encode_to(dest);
        self.decodes.encode_to(dest);
    }
}

impl Decode for MockData {
    fn decode<I: codec::Input>(value: &mut I) -> Result<Self, codec::Error> {
        let data = u32::decode(value)?;
        let filler = Vec::<u8>::decode(value)?;
        let decodes = bool::decode(value)?;
        if !decodes {
            return Err("Simulated decode failure.".into());
        }
        Ok(Self {
            data,
            filler,
            decodes,
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_peer(
    pen: AuthorityPen,
    addr: Addresses,
    n_msg: usize,
    large_message_interval: Option<usize>,
    corrupted_message_interval: Option<usize>,
    dialer: MockDialer,
    listener: MockListener,
    report: mpsc::UnboundedSender<(AuthorityId, usize)>,
    spawn_handle: SpawnTaskHandle,
) {
    let our_id = pen.authority_id();
    let (service, mut interface) = Service::new(dialer, listener, pen, spawn_handle);
    // run the service
    tokio::spawn(async {
        let (_exit, rx) = oneshot::channel();
        service.run(rx).await;
    });
    // start connecting with the peers
    let mut peer_ids = Vec::with_capacity(addr.len());
    for (id, addrs) in addr.into_iter() {
        interface.add_connection(id.clone(), addrs);
        peer_ids.push(id);
    }
    // peer main loop
    // we send random messages to random peers
    // a message is a number in range 0..n_msg
    // we also keep a list of messages received at least once
    // on receiving a message we report the total number of distinct messages received so far
    // the goal is to receive every message at least once
    tokio::spawn(async move {
        let mut received: HashSet<usize> = HashSet::with_capacity(n_msg);
        let mut send_ticker = tokio::time::interval(Duration::from_millis(5));
        let mut counter: usize = 0;
        loop {
            tokio::select! {
                _ = send_ticker.tick() => {
                    counter += 1;
                    // generate random message
                    let mut filler_size = 0;
                    if let Some(lmi) = large_message_interval && counter % lmi == 0 {
                        filler_size = TWICE_MAX_DATA_SIZE;
                    }
                    let mut decodes = true;
                    if let Some(cmi) = corrupted_message_interval && counter % cmi == 0 {
                        decodes = false;
                    }
                    let data: MockData = MockData::new(thread_rng().gen_range(0..n_msg) as u32, filler_size, decodes);
                    // choose a peer
                    let peer: AuthorityId = peer_ids[thread_rng().gen_range(0..peer_ids.len())].clone();
                    // send
                    interface.send(data, peer);
                },
                data = interface.next() => {
                    // receive the message
                    let data: MockData = data.expect("next should not be closed");
                    // mark the message as received, we do not care about sender's identity
                    received.insert(data.data as usize);
                    // report the number of received messages
                    report.unbounded_send((our_id.clone(), received.len())).expect("should send");
                },
            };
        }
    });
}

/// Takes O(n log n) rounds to finish, where n = n_peers * n_msg.
pub async fn scenario(
    n_peers: usize,
    n_msg: usize,
    broken_connection_interval: Option<usize>,
    large_message_interval: Option<usize>,
    corrupted_message_interval: Option<usize>,
    status_report_interval: Duration,
) {
    // create spawn_handle, we need to keep the task_manager
    let task_manager =
        TaskManager::new(Handle::current(), None).expect("should create TaskManager");
    let spawn_handle = task_manager.spawn_handle();
    // create peer identities
    info!(target: "validator-network", "generating keys...");
    let keys = random_keys(n_peers).await;
    info!(target: "validator-network", "done");
    // prepare and run the manager
    let (mut connection_manager, mut callers, addr) =
        UnreliableConnectionMaker::new(keys.keys().cloned().collect());
    tokio::spawn(async move {
        connection_manager.run(broken_connection_interval).await;
    });
    // channel for receiving status updates from spawned peers
    let (tx_report, mut rx_report) = mpsc::unbounded::<(AuthorityId, usize)>();
    let mut reports: BTreeMap<AuthorityId, usize> =
        keys.keys().cloned().map(|id| (id, 0)).collect();
    // spawn peers
    for (id, pen) in keys.into_iter() {
        let mut addr = addr.clone();
        // do not connect with itself
        addr.remove(&pen.authority_id());
        let (dialer, listener) = callers.remove(&id).expect("should contain all ids");
        spawn_peer(
            pen,
            addr,
            n_msg,
            large_message_interval,
            corrupted_message_interval,
            dialer,
            listener,
            tx_report.clone(),
            spawn_handle.clone(),
        );
    }
    let mut status_ticker = interval(status_report_interval);
    loop {
        tokio::select! {
            maybe_report = rx_report.next() => match maybe_report {
                Some((peer_id, peer_n_msg)) => {
                    reports.insert(peer_id, peer_n_msg);
                    if reports.values().all(|&x| x == n_msg) {
                        info!(target: "validator-network", "Peers received {:?} messages out of {}, finishing.", reports.values(), n_msg);
                        return;
                    }
                },
                None => panic!("should receive"),
            },
            _ = status_ticker.tick() => {
                info!(target: "validator-network", "Peers received {:?} messages out of {}.", reports.values(), n_msg);
            }
        };
    }
}

/// Takes O(n log n) rounds to finish, where n = n_peers * n_msg.
pub async fn scenario_with_timeout(
    n_peers: usize,
    n_msg: usize,
    broken_connection_interval: Option<usize>,
    large_message_interval: Option<usize>,
    corrupted_message_interval: Option<usize>,
    status_report_interval: Duration,
    scenario_timeout: Duration,
) -> Result<(), Elapsed> {
    timeout(
        scenario_timeout,
        scenario(
            n_peers,
            n_msg,
            broken_connection_interval,
            large_message_interval,
            corrupted_message_interval,
            status_report_interval,
        ),
    )
    .await
}
