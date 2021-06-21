use aleph_bft::{Index, KeyBox as _, NodeIndex, SignatureSet};
use codec::{Codec, Decode, Encode};
use futures::{channel::mpsc, stream::Stream, FutureExt, StreamExt};
use lru::LruCache;
use parking_lot::Mutex;
use sc_network::{multiaddr, Event, ExHashT, NetworkService, PeerId as ScPeerId, ReputationChange};
use sp_runtime::traits::Block as BlockT;
use std::{
    borrow::Cow,
    collections::HashMap,
    hash::{Hash, Hasher as StdHasher},
    iter,
    marker::PhantomData,
    pin::Pin,
    sync::Arc,
};

use log::{debug, trace, warn};

use crate::{finalization::SignableHash, Error, Hasher, MultiKeychain, SessionId, Signature};
use std::{fmt::Debug, future::Future};

#[cfg(test)]
mod tests;

// TODO below constants should be calculated based on the size of validators set for given session
const GOSSIP_FORWARD: usize = 5;
const SEND_FORWARD: usize = 2;
const CACHE_SIZE: usize = 10000;

#[derive(PartialEq, Eq, Copy, Clone, Debug, Hash)]
pub struct PeerId(ScPeerId);

impl From<PeerId> for ScPeerId {
    fn from(wrapper: PeerId) -> Self {
        wrapper.0
    }
}

impl From<ScPeerId> for PeerId {
    fn from(id: ScPeerId) -> Self {
        PeerId(id)
    }
}

impl Encode for PeerId {
    fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
        self.0.to_bytes().using_encoded(f)
    }
}

impl Decode for PeerId {
    fn decode<I: codec::Input>(value: &mut I) -> Result<Self, codec::Error> {
        let bytes = Vec::<u8>::decode(value)?;
        ScPeerId::from_bytes(&bytes)
            .map_err(|_| "PeerId not encoded with to_bytes".into())
            .map(|pid| pid.into())
    }
}

/// Name of the network protocol used by Aleph Zero. This is how messages
/// are subscribed to to ensure that we are gossiping and communicating with our
/// own network.
pub(crate) const ALEPH_PROTOCOL_NAME: &str = "/cardinals/aleph/1";

/// Abstraction over a network.
pub trait Network<B: BlockT>: Clone + Send + Sync + 'static {
    /// Returns a stream of events representing what happens on the network.
    fn event_stream(&self) -> Pin<Box<dyn Stream<Item = Event> + Send>>;

    /// Adjust the reputation of a node.
    fn _report_peer(&self, peer_id: PeerId, reputation: ReputationChange);

    /// Force-disconnect a peer.
    fn _disconnect_peer(&self, peer_id: PeerId, protocol: Cow<'static, str>);

    /// Send a message to a given peer.
    fn send_message(&self, peer_id: PeerId, protocol: Cow<'static, str>, message: Vec<u8>);

    /// Notify everyone we're connected to that we have the given block.
    /// This might be useful in the future.
    fn _announce(&self, block: B::Hash, associated_data: Option<Vec<u8>>);

    /// TODO: figure out what does this actually do...
    fn add_set_reserved(&self, who: PeerId, protocol: Cow<'static, str>);

    /// TODO: figure out what does this actually do...
    fn remove_set_reserved(&self, who: PeerId, protocol: Cow<'static, str>);

    // The PeerId of this node.
    fn peer_id(&self) -> PeerId;
}

impl<B: BlockT, H: ExHashT> Network<B> for Arc<NetworkService<B, H>> {
    fn event_stream(&self) -> Pin<Box<dyn Stream<Item = Event> + Send>> {
        Box::pin(NetworkService::event_stream(self, "network-gossip"))
    }

    fn _report_peer(&self, peer_id: PeerId, reputation: ReputationChange) {
        NetworkService::report_peer(self, peer_id.into(), reputation);
    }

    fn _disconnect_peer(&self, peer_id: PeerId, protocol: Cow<'static, str>) {
        NetworkService::disconnect_peer(self, peer_id.into(), protocol)
    }

    fn send_message(&self, peer_id: PeerId, protocol: Cow<'static, str>, message: Vec<u8>) {
        NetworkService::write_notification(self, peer_id.into(), protocol, message)
    }

    fn _announce(&self, block: B::Hash, associated_data: Option<Vec<u8>>) {
        NetworkService::announce_block(self, block, associated_data)
    }

    fn add_set_reserved(&self, who: PeerId, protocol: Cow<'static, str>) {
        let addr =
            iter::once(multiaddr::Protocol::P2p(who.0.into())).collect::<multiaddr::Multiaddr>();
        let result =
            NetworkService::add_peers_to_reserved_set(self, protocol, iter::once(addr).collect());
        if let Err(e) = result {
            log::error!(target: "afa", "add_set_reserved failed: {}", e);
        }
    }

    fn remove_set_reserved(&self, who: PeerId, protocol: Cow<'static, str>) {
        let addr =
            iter::once(multiaddr::Protocol::P2p(who.0.into())).collect::<multiaddr::Multiaddr>();
        let result = NetworkService::remove_peers_from_reserved_set(
            self,
            protocol,
            iter::once(addr).collect(),
        );
        if let Err(e) = result {
            log::error!(target: "afa", "remove_set_reserved failed: {}", e);
        }
    }

    fn peer_id(&self) -> PeerId {
        (*self.local_peer_id()).into()
    }
}

#[derive(Debug)]
struct PeerInfo {
    authentications: HashMap<SessionId, NodeIndex>,
}

impl PeerInfo {
    fn new() -> Self {
        PeerInfo {
            authentications: HashMap::new(),
        }
    }

    fn authenticated_for(&self, session_id: &SessionId) -> bool {
        self.authentications.get(session_id).is_some()
    }

    fn authenticate(&mut self, session_id: SessionId, node_id: NodeIndex) {
        self.authentications.insert(session_id, node_id);
    }

    fn iter(&self) -> impl Iterator<Item = (&SessionId, &NodeIndex)> {
        self.authentications.iter()
    }
}

struct Peers {
    all_peers: HashMap<PeerId, PeerInfo>,
    to_peer: HashMap<SessionId, HashMap<NodeIndex, PeerId>>,
}

impl Peers {
    fn new() -> Self {
        Peers {
            all_peers: HashMap::new(),
            to_peer: HashMap::new(),
        }
    }

    fn insert(&mut self, peer: PeerId) {
        self.all_peers.insert(peer, PeerInfo::new());
    }

    fn is_authenticated(&self, peer: &PeerId, session_id: &SessionId) -> bool {
        match self.all_peers.get(peer) {
            Some(info) => info.authenticated_for(session_id),
            None => false,
        }
    }

    fn authenticate(&mut self, peer: &PeerId, session_id: SessionId, node_id: NodeIndex) {
        if self.all_peers.get(peer).is_none() {
            self.insert(*peer);
        }
        self.all_peers
            .entry(*peer)
            .or_insert_with(PeerInfo::new)
            .authenticate(session_id, node_id);
        self.to_peer
            .entry(session_id)
            .or_insert_with(HashMap::new)
            .insert(node_id, *peer);
    }

    fn remove(&mut self, peer: &PeerId) {
        if let Some(peer_info) = self.all_peers.remove(peer) {
            for (session_id, node_id) in peer_info.iter() {
                self.to_peer.entry(*session_id).and_modify(|hm| {
                    hm.remove(node_id);
                });
            }
        }
        self.to_peer.retain(|_, hm| !hm.is_empty());
    }

    fn peers_authenticated_for(
        &self,
        session_id: SessionId,
    ) -> impl Iterator<Item = &PeerId> + Clone {
        self.to_peer
            .get(&session_id)
            .into_iter()
            .map(|hm| hm.values())
            .flatten()
    }

    fn get(&self, session_id: SessionId, node_id: NodeIndex) -> Option<&PeerId> {
        self.to_peer.get(&session_id)?.get(&node_id)
    }

    fn get_rand(
        &self,
        session_id: SessionId,
        amount: usize,
        offset: usize,
    ) -> impl Iterator<Item = &PeerId> {
        let skip = if let Some(n_peers) = self.to_peer.get(&session_id).map(|hm| hm.len()) {
            offset % n_peers
        } else {
            0
        };
        self.peers_authenticated_for(session_id)
            .cycle()
            .skip(skip)
            .take(amount)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum SessionStatus {
    InProgress,
    Terminated,
}

#[derive(Clone, Copy, Encode, Decode)]
pub(crate) enum Recipient<T: Clone + Encode + Decode> {
    All,
    Target(T),
}

#[derive(Clone, Encode, Decode)]
struct AuthData {
    session_id: SessionId,
    peer_id: PeerId,
    node_id: NodeIndex,
}

#[derive(Clone, Encode, Decode)]
enum MetaMessage {
    Authentication(AuthData, Signature),
    AuthenticationRequest(SessionId),
}

type MessageIndex = u64;

#[derive(Clone, Encode, Decode)]
struct DataMessage<D> {
    session_id: SessionId,
    index: MessageIndex,
    data: D,
    recipient: Recipient<NodeIndex>,
}

impl<D: Encode> Hash for DataMessage<D> {
    fn hash<H: StdHasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.data.encode().hash(state);
    }
}

impl<D: Encode> PartialEq for DataMessage<D> {
    fn eq(&self, other: &DataMessage<D>) -> bool {
        self.index == other.index && self.data.encode() == other.data.encode()
    }
}

impl<D: Encode> Eq for DataMessage<D> {}

#[derive(Clone, Encode, Decode)]
enum InternalMessage<D: Clone + Encode + Decode> {
    Meta(MetaMessage),
    Data(DataMessage<D>),
}

struct SessionData<D> {
    pub(crate) data_for_user: mpsc::UnboundedSender<D>,
    pub(crate) status: SessionStatus,
    pub(crate) keychain: MultiKeychain,
    messages: LruCache<DataMessage<D>, ()>,
    messages_index: MessageIndex,
    auth_data: AuthData,
    auth_signature: Signature,
}

#[derive(Clone, Encode, Decode)]
enum SessionCommand<D: Clone + Encode + Decode> {
    Meta(MetaMessage, Recipient<PeerId>),
    Data(SessionId, D, Recipient<NodeIndex>),
}

impl<D: Clone + Codec> SessionCommand<D> {
    fn map<E: Clone + Codec, F: FnOnce(D) -> E>(self, f: F) -> SessionCommand<E> {
        use SessionCommand::*;
        match self {
            Meta(message, recipient) => Meta(message, recipient),
            Data(session_id, data, recipient) => Data(session_id, f(data), recipient),
        }
    }
}

pub(crate) struct SessionManager<D: Clone + Codec> {
    peer_id: PeerId,
    sessions: Arc<Mutex<HashMap<SessionId, SessionData<D>>>>,
    commands_for_session: mpsc::UnboundedSender<SessionCommand<D>>,
}

impl<D: Clone + Codec> SessionManager<D> {
    pub(crate) async fn start_session(
        &self,
        session_id: SessionId,
        keychain: MultiKeychain,
    ) -> DataNetwork<D> {
        let auth_data = AuthData {
            session_id,
            peer_id: self.peer_id,
            node_id: keychain.index(),
        };
        let signature = keychain.sign(&auth_data.encode()).await;
        let (data_for_user, data_from_network) = mpsc::unbounded();
        let session_data = SessionData {
            data_for_user,
            status: SessionStatus::InProgress,
            keychain,
            messages: LruCache::new(CACHE_SIZE),
            messages_index: 0,
            auth_data: auth_data.clone(),
            auth_signature: signature.clone(),
        };
        self.sessions.lock().insert(session_id, session_data);
        if let Err(e) = self
            .commands_for_session
            .unbounded_send(SessionCommand::Meta(
                MetaMessage::Authentication(auth_data, signature),
                Recipient::All,
            ))
        {
            log::error!(target: "afa", "sending auth command failed in new session: {}", e);
        }
        DataNetwork::new(
            session_id,
            data_from_network,
            self.commands_for_session.clone(),
        )
    }
}

pub(crate) struct ConsensusNetwork<D: Clone + Encode + Decode, B: BlockT, N: Network<B> + Clone> {
    //TODO: some optimizations can be made by changing Mutex to RwLock
    network: N,
    protocol: Cow<'static, str>,

    /// Outgoing events to the consumer.
    sessions: Arc<Mutex<HashMap<SessionId, SessionData<D>>>>,

    commands_for_session: mpsc::UnboundedSender<SessionCommand<D>>,
    commands_from_user: mpsc::UnboundedReceiver<SessionCommand<D>>,

    peers: Arc<Mutex<Peers>>,
    _phantom: PhantomData<B>,
}

impl<D, B, N> ConsensusNetwork<D, B, N>
where
    D: Clone + Encode + Decode,
    B: BlockT + 'static,
    N: Network<B> + Clone,
{
    /// Create a new instance.
    pub(crate) fn new(network: N, protocol: Cow<'static, str>) -> Self {
        let (commands_for_session, commands_from_user) = mpsc::unbounded();
        ConsensusNetwork {
            network,
            protocol,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            commands_for_session,
            commands_from_user,
            peers: Arc::new(Mutex::new(Peers::new())),
            _phantom: PhantomData,
        }
    }

    pub(crate) fn session_manager(&self) -> SessionManager<D> {
        SessionManager {
            peer_id: self.network.peer_id(),
            sessions: self.sessions.clone(),
            commands_for_session: self.commands_for_session.clone(),
        }
    }

    fn next_message_index(&self, session_id: SessionId) -> MessageIndex {
        let mut sessions = self.sessions.lock();
        let mut index = sessions
            .get_mut(&session_id)
            .expect("Session has stared.")
            .messages_index;
        index += 1;

        index
    }

    fn send_message(&self, peer_id: &PeerId, message: InternalMessage<D>) {
        self.network
            .send_message(*peer_id, self.protocol.clone(), message.encode());
    }

    fn send_message_rand(
        &self,
        session_id: SessionId,
        message: InternalMessage<D>,
        amount: usize,
        offset: usize,
    ) {
        for peer_id in self.peers.lock().get_rand(session_id, amount, offset) {
            self.send_message(peer_id, message.clone());
        }
    }

    fn forward_to_peers(&self, message: DataMessage<D>) {
        let DataMessage {
            session_id,
            index,
            recipient,
            ..
        } = message;
        let message = InternalMessage::Data(message);

        match recipient {
            Recipient::All => {
                self.send_message_rand(session_id, message, GOSSIP_FORWARD, index as usize)
            }
            Recipient::Target(node_id) => {
                if let Some(peer_id) = self.peers.lock().get(session_id, node_id) {
                    self.send_message(peer_id, message);
                } else {
                    self.send_message_rand(session_id, message, SEND_FORWARD, index as usize);
                }
            }
        }
    }

    fn send_to_user(&self, session_id: SessionId, data: D, session_data: &mut SessionData<D>) {
        if let Err(e) = session_data.data_for_user.unbounded_send(data) {
            //TODO: need to write some logic on when an session should be terminated and make sure
            // that there are no issues with synchronization when terminating.
            session_data.status = SessionStatus::Terminated;
            debug!(target: "afa", "Error {:?} when passing a message event to session {:?}.", e, session_id);
        }
    }

    fn authenticate_to(&self, session_data: &SessionData<D>, peer_id: PeerId) {
        self.commands_for_session
            .unbounded_send(SessionCommand::Meta(
                MetaMessage::Authentication(
                    session_data.auth_data.clone(),
                    session_data.auth_signature.clone(),
                ),
                Recipient::Target(peer_id),
            ))
            .expect("Sending commands to session should work.");
    }

    fn on_incoming_meta(&self, message: MetaMessage, peer_id: PeerId) {
        use MetaMessage::*;
        match message {
            Authentication(auth_data, signature) => {
                // Avoids peers claiming other peers represent their node, which could lead to a
                // DDoS.
                if peer_id == auth_data.peer_id {
                    self.on_incoming_authentication(auth_data, signature);
                } else {
                    trace!(target: "afa", "Peer {:?} attempting to authenticate as peer {:?}.", peer_id, auth_data.peer_id);
                }
            }
            AuthenticationRequest(session_id) => {
                if let Some(session_data) = self.sessions.lock().get(&session_id) {
                    self.authenticate_to(session_data, peer_id);
                } else {
                    trace!(target: "afa", "Received authentication request for unknown session: {:?}.", session_id);
                }
            }
        }
    }

    fn on_incoming_data(&self, message: DataMessage<D>) {
        let session_id = message.session_id;
        let mut sessions = self.sessions.lock();
        if let Some(session_data) = sessions.get_mut(&session_id) {
            if session_data.status == SessionStatus::InProgress {
                if session_data.messages.contains(&message) {
                    trace!(target: "afa", "Received data with old index in session {:?}.", session_id);
                    return;
                } else {
                    session_data.messages.put(message.clone(), ());
                }
                match message.recipient {
                    Recipient::Target(node_id) if node_id == session_data.auth_data.node_id => {
                        self.send_to_user(session_id, message.data, session_data);
                    }
                    Recipient::All => {
                        self.send_to_user(session_id, message.data.clone(), session_data);
                        self.forward_to_peers(message);
                    }
                    _ => {
                        self.forward_to_peers(message);
                    }
                }
            }
        }
    }

    fn on_incoming_authentication(&self, auth_data: AuthData, signature: Signature) {
        let enc_auth_data = auth_data.encode();
        let AuthData {
            session_id,
            peer_id,
            node_id,
        } = auth_data;
        if let Some(session_data) = self.sessions.lock().get(&session_id) {
            if session_data
                .keychain
                .verify(&enc_auth_data, &signature, node_id)
            {
                self.peers
                    .lock()
                    .authenticate(&peer_id, session_id, node_id);
            }
        }
    }

    fn on_incoming_message(&self, peer_id: PeerId, raw_message: Vec<u8>) {
        use InternalMessage::*;
        match InternalMessage::<D>::decode(&mut &raw_message[..]) {
            Ok(Data(message)) => {
                let session_id = message.session_id;
                // Accept data only from authenticated peers. Rush is robust enough that this is
                // not strictly necessary, but it doesn't hurt.
                // TODO we may relax this condition if we want to allow nonvalidators to help in gossip
                if self.peers.lock().is_authenticated(&peer_id, &session_id) {
                    self.on_incoming_data(message);
                } else {
                    trace!(target: "afa", "Received unauthenticated message from {:?} for session {:?}, requesting authentication.", peer_id, session_id);
                    self.commands_for_session
                        .unbounded_send(SessionCommand::Meta(
                            MetaMessage::AuthenticationRequest(session_id),
                            Recipient::Target(peer_id),
                        ))
                        .expect("Sending commands to session should work.");
                }
            }
            Ok(Meta(message)) => {
                self.on_incoming_meta(message, peer_id);
            }
            Err(e) => {
                debug!(target: "afa", "Error decoding message: {}", e);
            }
        }
    }

    fn on_command(&self, sc: SessionCommand<D>) {
        use SessionCommand::*;
        match sc {
            Meta(message, recipient) => {
                let message = InternalMessage::Meta(message);
                match recipient {
                    Recipient::All => {
                        for (peer_id, _) in self.peers.lock().all_peers.iter() {
                            self.send_message(peer_id, message.clone());
                        }
                    }
                    Recipient::Target(peer_id) => self.send_message(&peer_id, message),
                }
            }
            Data(session_id, data, recipient) => {
                let index = self.next_message_index(session_id);
                let data_message = DataMessage {
                    session_id,
                    index,
                    data,
                    recipient,
                };
                let message = InternalMessage::Data(data_message);
                if let Recipient::Target(node_id) = recipient {
                    if let Some(peer) = self.peers.lock().get(session_id, node_id) {
                        self.send_message(peer, message);
                        return;
                    }
                }
                for peer_id in self.peers.lock().peers_authenticated_for(session_id) {
                    self.send_message(peer_id, message.clone());
                }
            }
        }
    }

    fn on_peer_connected(&self, peer_id: PeerId) {
        self.peers.lock().insert(peer_id);
        for (_, session_data) in self.sessions.lock().iter() {
            self.authenticate_to(session_data, peer_id);
        }
    }

    fn on_peer_disconnected(&self, peer_id: &PeerId) {
        self.peers.lock().remove(peer_id);
    }

    pub async fn run(mut self) {
        let mut network_event_stream = self.network.event_stream();

        loop {
            tokio::select! {
                maybe_event = network_event_stream.next() => {
                        if let Some(event) = maybe_event {
                            match event {
                                Event::SyncConnected { remote } => {
                                    // TODO: understand what does this do
                                    self.network.add_set_reserved(remote.into(), self.protocol.clone());
                                }
                                Event::SyncDisconnected { remote } => {
                                    // TODO: understand what does this do
                                    self.network
                                        .remove_set_reserved(remote.into(), self.protocol.clone());
                                }
                                Event::NotificationStreamOpened {
                                    remote,
                                    protocol,
                                    role: _,
                                    negotiated_fallback: _,
                                } => {
                                    if protocol != self.protocol {
                                        continue;
                                    }
                                    self.on_peer_connected(remote.into());
                                }
                                Event::NotificationStreamClosed { remote, protocol } => {
                                    if protocol != self.protocol {
                                        continue;
                                    }
                                    self.on_peer_disconnected(&remote.into());
                                }
                                Event::NotificationsReceived { remote, messages } => {
                                    for (protocol, data) in messages.into_iter() {
                                        if protocol == self.protocol {
                                            self.on_incoming_message(remote.into(), data.to_vec());
                                        }
                                    }
                                }
                                Event::Dht(_) => {
                                    // TODO: add support, if relevant
                                }
                            }
                        }
                        else {
                            //TODO: The network event stream closed, what shall we do?
                            break;
                        }
                },
                maybe_cmd = self.commands_from_user.next() => {
                    if let Some(cmd) = maybe_cmd {
                        self.on_command(cmd);
                    } else {
                        break;
                    }
                }
            }

            self.sessions
                .lock()
                .retain(|_, data| data.status == SessionStatus::InProgress);
        }
    }
}

pub(crate) type AlephNetworkData<B> =
    aleph_bft::NetworkData<Hasher, <B as BlockT>::Hash, Signature, SignatureSet<Signature>>;

pub(crate) type RmcNetworkData<B> =
    aleph_bft::rmc::Message<SignableHash<<B as BlockT>::Hash>, Signature, SignatureSet<Signature>>;

#[derive(Clone, Encode, Decode)]
pub(crate) enum NetworkData<B: BlockT> {
    Aleph(AlephNetworkData<B>),
    Rmc(RmcNetworkData<B>),
}

pub(crate) struct DataNetwork<D: Clone + Codec> {
    session_id: SessionId,
    data_from_consensus_network: mpsc::UnboundedReceiver<D>,
    commands_for_consensus_network: mpsc::UnboundedSender<SessionCommand<D>>,
}

impl<D: Clone + Codec> DataNetwork<D> {
    fn new(
        session_id: SessionId,
        data_from_consensus_network: mpsc::UnboundedReceiver<D>,
        commands_for_consensus_network: mpsc::UnboundedSender<SessionCommand<D>>,
    ) -> Self {
        DataNetwork {
            session_id,
            data_from_consensus_network,
            commands_for_consensus_network,
        }
    }

    pub(crate) fn send(&self, data: D, recipient: Recipient<NodeIndex>) -> Result<(), Error> {
        let sc = SessionCommand::Data(self.session_id, data, recipient);
        self.commands_for_consensus_network
            .unbounded_send(sc)
            .map_err(|_| Error::SendData)
    }

    pub(crate) async fn next(&mut self) -> Option<D> {
        self.data_from_consensus_network.next().await
    }
}

pub(crate) struct AlephNetwork<B: BlockT> {
    inner: DataNetwork<AlephNetworkData<B>>,
}

impl<B: BlockT> AlephNetwork<B> {
    pub(crate) fn new(inner: DataNetwork<AlephNetworkData<B>>) -> Self {
        AlephNetwork { inner }
    }
}

#[async_trait::async_trait]
impl<B: BlockT> aleph_bft::Network<Hasher, B::Hash, Signature, SignatureSet<Signature>>
    for AlephNetwork<B>
{
    type Error = Error;

    fn send(&self, data: AlephNetworkData<B>, node: NodeIndex) -> Result<(), Self::Error> {
        self.inner.send(data, Recipient::Target(node))
    }

    fn broadcast(&self, data: AlephNetworkData<B>) -> Result<(), Self::Error> {
        self.inner.send(data, Recipient::All)
    }

    async fn next_event(&mut self) -> Option<AlephNetworkData<B>> {
        self.inner.next().await
    }
}

pub(crate) struct RmcNetwork<B: BlockT> {
    inner: DataNetwork<RmcNetworkData<B>>,
}

impl<B: BlockT> RmcNetwork<B> {
    pub(crate) fn new(inner: DataNetwork<RmcNetworkData<B>>) -> Self {
        RmcNetwork { inner }
    }
    pub(crate) fn send(
        &self,
        data: RmcNetworkData<B>,
        recipient: Recipient<NodeIndex>,
    ) -> Result<(), Error> {
        self.inner.send(data, recipient)
    }
    pub(crate) async fn next(&mut self) -> Option<RmcNetworkData<B>> {
        self.inner.next().await
    }
}

pub(crate) fn split_network<B: BlockT>(
    data_network: DataNetwork<NetworkData<B>>,
) -> (AlephNetwork<B>, RmcNetwork<B>, impl Future<Output = ()>) {
    let (aleph_data_tx, aleph_data_rx) = mpsc::unbounded();
    let (rmc_data_tx, rmc_data_rx) = mpsc::unbounded();
    let (aleph_cmd_tx, aleph_cmd_rx) = mpsc::unbounded();
    let (rmc_cmd_tx, rmc_cmd_rx) = mpsc::unbounded();
    let aleph_network = AlephNetwork::new(DataNetwork::new(
        data_network.session_id,
        aleph_data_rx,
        aleph_cmd_tx,
    ));
    let rmc_network = RmcNetwork::new(DataNetwork::new(
        data_network.session_id,
        rmc_data_rx,
        rmc_cmd_tx,
    ));
    let mut data_from_consensus_network = data_network.data_from_consensus_network;
    let forward_data = async move {
        loop {
            match data_from_consensus_network.next().await {
                None => break,
                Some(NetworkData::Aleph(data)) => {
                    if let Err(e) = aleph_data_tx.unbounded_send(data) {
                        warn!(target: "afa", "unable to send data via aleph network {}", e);
                    }
                }
                Some(NetworkData::Rmc(data)) => {
                    if let Err(e) = rmc_data_tx.unbounded_send(data) {
                        warn!(target: "afa", "unable to send data via rmc network {}", e);
                    }
                }
            }
        }
    };
    let cmd_tx = data_network.commands_for_consensus_network;
    let forward_aleph_cmd = {
        let cmd_tx = cmd_tx.clone();
        aleph_cmd_rx
            .map(|cmd| Ok(cmd.map(NetworkData::Aleph)))
            .forward(cmd_tx)
            .map(|res| {
                if let Err(e) = res {
                    warn!(target: "afa", "error forwarding aleph commands: {}", e);
                }
            })
    };
    let forward_rmc_cmd = {
        rmc_cmd_rx
            .map(|cmd| Ok(cmd.map(NetworkData::Rmc)))
            .forward(cmd_tx)
            .map(|res| {
                if let Err(e) = res {
                    warn!(target: "afa", "error forwarding rmc commands: {}", e);
                }
            })
    };
    let forwards = futures::future::join3(forward_data, forward_aleph_cmd, forward_rmc_cmd)
        .map(|((), (), ())| ());
    (aleph_network, rmc_network, forwards)
}
