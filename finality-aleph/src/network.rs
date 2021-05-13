use codec::{Decode, Encode};
use futures::{channel::mpsc, stream::Stream, StreamExt};
use parking_lot::Mutex;
use rush::{NetworkCommand, NetworkEvent};
use sc_network::{multiaddr, Event, ExHashT, NetworkService, PeerId, ReputationChange};
use sp_runtime::traits::Block as BlockT;
use std::{borrow::Cow, collections::HashMap, iter, marker::PhantomData, pin::Pin, sync::Arc};

use log::debug;

use crate::{AuthorityId, Error, SessionId};

#[cfg(test)]
mod tests;

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
}

impl<B: BlockT, H: ExHashT> Network<B> for Arc<NetworkService<B, H>> {
    fn event_stream(&self) -> Pin<Box<dyn Stream<Item = Event> + Send>> {
        //Arc::new(Mutex::new(NetworkService::event_stream(self, "aleph-network")))
        Box::pin(NetworkService::event_stream(self, "network-gossip"))
    }

    fn _report_peer(&self, peer_id: PeerId, reputation: ReputationChange) {
        NetworkService::report_peer(self, peer_id, reputation);
    }

    fn _disconnect_peer(&self, peer_id: PeerId, protocol: Cow<'static, str>) {
        NetworkService::disconnect_peer(self, peer_id, protocol)
    }

    fn send_message(&self, peer_id: PeerId, protocol: Cow<'static, str>, message: Vec<u8>) {
        NetworkService::write_notification(self, peer_id, protocol, message)
    }

    fn _announce(&self, block: B::Hash, associated_data: Option<Vec<u8>>) {
        NetworkService::announce_block(self, block, associated_data)
    }

    fn add_set_reserved(&self, who: PeerId, protocol: Cow<'static, str>) {
        let addr =
            iter::once(multiaddr::Protocol::P2p(who.into())).collect::<multiaddr::Multiaddr>();
        let result =
            NetworkService::add_peers_to_reserved_set(self, protocol, iter::once(addr).collect());
        if let Err(e) = result {
            log::error!(target: "afa", "add_set_reserved failed: {}", e);
        }
    }

    fn remove_set_reserved(&self, who: PeerId, protocol: Cow<'static, str>) {
        let addr =
            iter::once(multiaddr::Protocol::P2p(who.into())).collect::<multiaddr::Multiaddr>();
        let result = NetworkService::remove_peers_from_reserved_set(
            self,
            protocol,
            iter::once(addr).collect(),
        );
        if let Err(e) = result {
            log::error!(target: "afa", "remove_set_reserved failed: {}", e);
        }
    }
}

use rand::{seq::SliceRandom, thread_rng};

#[derive(Debug)]
pub(crate) struct PeerInfo {}

impl PeerInfo {
    fn new() -> Self {
        PeerInfo {}
    }
}

#[derive(Debug)]
struct Peers {
    pub(crate) peers: HashMap<PeerId, PeerInfo>,
}

impl Peers {
    pub(crate) fn new() -> Self {
        Peers {
            peers: HashMap::new(),
        }
    }
    pub(crate) fn insert(&mut self, peer: PeerId) {
        self.peers.insert(peer, PeerInfo::new());
    }

    pub(crate) fn remove(&mut self, peer: &PeerId) {
        self.peers.remove(peer);
    }

    pub(crate) fn _contains(&self, peer: &PeerId) -> bool {
        self.peers.contains_key(peer)
    }

    //TODO: optimize this (it does not need to be perfectly random, if this helps)
    pub(crate) fn sample_random(&self) -> Option<PeerId> {
        let peers: Vec<&PeerId> = self.peers.keys().collect();
        let mut rng = thread_rng();
        peers.choose(&mut rng).cloned().cloned()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&PeerId, &PeerInfo)> {
        self.peers.iter()
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum SessionStatus {
    InProgress,
    Terminated,
}

struct SessionData {
    pub(crate) net_event_tx: mpsc::UnboundedSender<NetworkEvent>,
    pub(crate) status: SessionStatus,
    pub(crate) _authorities: Vec<AuthorityId>,
}

#[derive(Debug, Clone, Encode, Decode)]
struct SessionCommand {
    session_id: SessionId,
    command: NetworkCommand,
}

#[derive(Debug, Encode, Decode)]
struct SessionMessage {
    session_id: SessionId,
    message: Vec<u8>,
}

pub(crate) struct RushNetwork {
    session_id: SessionId,
    net_event_rx: mpsc::UnboundedReceiver<NetworkEvent>,
    net_command_tx: mpsc::UnboundedSender<SessionCommand>,
}

#[async_trait::async_trait]
impl rush::Network for RushNetwork {
    type Error = Error;

    fn send(&self, command: NetworkCommand) -> Result<(), Self::Error> {
        let sc = SessionCommand {
            session_id: self.session_id,
            command,
        };
        // TODO add better error conversion
        self.net_command_tx
            .unbounded_send(sc)
            .map_err(|_| Error::SendData)
    }

    async fn next_event(&mut self) -> Option<NetworkEvent> {
        self.net_event_rx.next().await
    }
}

pub(crate) struct ConsensusNetwork<B: BlockT, N: Network<B> + Clone> {
    //TODO: some optimizations can be made by changing Mutex to RwLock
    network: N,
    protocol: Cow<'static, str>,

    /// Outgoing events to the consumer.
    sessions: Arc<Mutex<HashMap<SessionId, SessionData>>>,

    net_command_tx: mpsc::UnboundedSender<SessionCommand>,
    net_command_rx: mpsc::UnboundedReceiver<SessionCommand>,

    peers: Arc<Mutex<Peers>>,
    phantom: PhantomData<B>,
}

pub(crate) struct SessionManagar {
    sessions: Arc<Mutex<HashMap<SessionId, SessionData>>>,
    net_command_tx: mpsc::UnboundedSender<SessionCommand>,
}

impl SessionManagar {
    // NOTE: later this will also need access to KeyStore :/ (for Reliable Broadcast)
    pub(crate) fn start_session(
        &self,
        session_id: SessionId,
        _authorities: Vec<AuthorityId>,
    ) -> RushNetwork {
        let (net_event_tx, net_event_rx) = mpsc::unbounded();
        let session_data = SessionData {
            net_event_tx,
            status: SessionStatus::InProgress,
            _authorities,
        };
        self.sessions.lock().insert(session_id, session_data);
        RushNetwork {
            session_id,
            net_event_rx,
            net_command_tx: self.net_command_tx.clone(),
        }
    }
}

impl<B: BlockT + 'static, N: Network<B> + Clone> ConsensusNetwork<B, N> {
    /// Create a new instance.
    pub(crate) fn new(network: N, protocol: impl Into<Cow<'static, str>>) -> Self {
        let protocol = protocol.into();
        let (net_command_tx, net_command_rx) = mpsc::unbounded();
        ConsensusNetwork {
            network,
            protocol,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            net_command_tx,
            net_command_rx,
            peers: Arc::new(Mutex::new(Peers::new())),
            phantom: PhantomData,
        }
    }

    pub(crate) fn session_manager(&self) -> SessionManagar {
        SessionManagar {
            sessions: self.sessions.clone(),
            net_command_tx: self.net_command_tx.clone(),
        }
    }

    fn sample_random_peer(&self) -> Option<PeerId> {
        self.peers.lock().sample_random()
    }

    fn send_message(&self, peer_id: PeerId, session_id: SessionId, message: &Vec<u8>) {
        self.network.send_message(
            peer_id,
            self.protocol.clone(),
            (session_id, message).encode(),
        );
    }

    fn on_incoming_message(&self, peer_id: PeerId, raw_message: Vec<u8>) {
        match <(SessionId, Vec<u8>)>::decode(&mut &raw_message[..]) {
            Ok((session_id, message)) => {
                let mut sessions = self.sessions.lock();
                let maybe_data = sessions.get_mut(&session_id);
                if let Some(session_data) = maybe_data {
                    if session_data.status == SessionStatus::InProgress {
                        if let Err(e) =
                            session_data
                                .net_event_tx
                                .unbounded_send(NetworkEvent::MessageReceived(
                                    message,
                                    peer_id.to_bytes(),
                                ))
                        {
                            //TODO: need to write some logic on when an session should be terminated and make sure
                            // that there are no issues with synchronization when terminating.
                            session_data.status = SessionStatus::Terminated;
                            debug!(target: "afa", "Error {:?} when passing a message event to session {:?}.", e, session_id);
                        }
                    }
                }
            }
            Err(e) => {
                debug!(target: "afa", "Error decoding message: {}", e);
            }
        }
    }

    fn on_command(&self, sc: SessionCommand) {
        match sc.command {
            NetworkCommand::SendToAll(message) => {
                debug!(target: "afa", "Sending message to {} peers.", self.peers.lock().peers.len());
                for (peer_id, _) in self.peers.lock().iter() {
                    self.send_message(*peer_id, sc.session_id, &message);
                }
            }
            NetworkCommand::ReliableBroadcast(message) => {
                //TODO!!!!! This should be a real RBC, not multicast like now
                debug!(target: "afa", "Sending RBC message to {} peers.", self.peers.lock().peers.len());
                for (peer_id, _) in self.peers.lock().iter() {
                    self.send_message(*peer_id, sc.session_id, &message);
                }
            }
            NetworkCommand::SendToPeer(message, peer_id_bytes) => {
                self.send_message(
                    PeerId::from_bytes(&peer_id_bytes[..])
                        .expect("peer_id was encoded with `to_bytes`"),
                    sc.session_id,
                    &message,
                );
            }
            NetworkCommand::SendToRandPeer(message) => {
                if let Some(peer_id) = self.sample_random_peer() {
                    self.send_message(peer_id, sc.session_id, &message);
                } else {
                    debug!(target: "afa", "Attempting to send a message, but no connected peers.");
                }
            }
        }
    }

    fn on_peer_connected(&self, peer_id: PeerId) {
        self.peers.lock().insert(peer_id);
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
                                    self.network.add_set_reserved(remote, self.protocol.clone());
                                }
                                Event::SyncDisconnected { remote } => {
                                    // TODO: understand what does this do
                                    self.network
                                        .remove_set_reserved(remote, self.protocol.clone());
                                }
                                Event::NotificationStreamOpened {
                                    remote,
                                    protocol,
                                    role: _,
                                } => {
                                    if protocol != self.protocol {
                                        continue;
                                    }
                                    self.on_peer_connected(remote);
                                }
                                Event::NotificationStreamClosed { remote, protocol } => {
                                    if protocol != self.protocol {
                                        continue;
                                    }
                                    self.on_peer_disconnected(&remote);
                                }
                                Event::NotificationsReceived { remote, messages } => {
                                    for (protocol, data) in messages.into_iter() {
                                        if protocol == self.protocol {
                                            self.on_incoming_message(remote, data.to_vec());
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
                maybe_cmd = self.net_command_rx.next() => {
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
