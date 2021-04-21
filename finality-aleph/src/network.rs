use codec::{Decode, Encode};
use futures::{channel::mpsc, stream::Stream, StreamExt};
use parking_lot::Mutex;
use sc_network::{multiaddr, Event, ExHashT, NetworkService, PeerId, ReputationChange};
use sp_runtime::traits::Block as BlockT;
use std::{borrow::Cow, collections::HashMap, iter, pin::Pin, sync::Arc};

use log::debug;

use crate::{
    hash::Hash,
    messages::{ConsensusMessage, NetworkMessage},
    AuthorityId, EpochId,
};

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
            log::error!(target: "network", "add_set_reserved failed: {}", e);
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
            log::error!(target: "network", "remove_set_reserved failed: {}", e);
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
pub(crate) struct Peers {
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

#[derive(Clone, Debug)]
pub(crate) enum NetworkCommand<B: BlockT, H: Hash> {
    SendToAll(NetworkMessage<B, H>),
    SendToPeer(NetworkMessage<B, H>, PeerId),
    SendToRandPeer(NetworkMessage<B, H>),
}

#[derive(Clone, Debug)]
pub(crate) enum NetworkEvent<B: BlockT, H: Hash> {
    MessageReceived(ConsensusMessage<B, H>, PeerId),
    PeerConnected(PeerId),
    PeerDisconnected(PeerId),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum EpochStatus {
    InProgress,
    Terminated,
}

pub(crate) struct EpochData<B: BlockT, H: Hash> {
    pub(crate) tx: mpsc::UnboundedSender<NetworkEvent<B, H>>,
    pub(crate) status: EpochStatus,
    pub(crate) _authorities: Vec<AuthorityId>,
}

#[derive(Clone)]
pub(crate) struct ConsensusNetwork<B: BlockT, H: Hash, N: Network<B> + Clone> {
    //TODO: some optimizations can be made by changing Mutex to RwLock
    network: N,
    protocol: Cow<'static, str>,

    /// Outgoing events to the consumer.
    epochs: Arc<Mutex<HashMap<EpochId, EpochData<B, H>>>>,

    peers: Arc<Mutex<Peers>>,
}

impl<B: BlockT + 'static, H: Hash, N: Network<B> + Clone> ConsensusNetwork<B, H, N> {
    /// Create a new instance.
    pub fn new(network: N, protocol: impl Into<Cow<'static, str>>) -> Self {
        let protocol = protocol.into();
        ConsensusNetwork {
            network,
            protocol,
            epochs: Arc::new(Mutex::new(HashMap::new())),
            peers: Arc::new(Mutex::new(Peers::new())),
        }
    }

    // NOTE: later this will also need access to KeyStore :/ (for Reliable Broadcast)
    pub fn start_epoch(
        &self,
        epoch_id: EpochId,
        _authorities: Vec<AuthorityId>,
    ) -> mpsc::UnboundedReceiver<NetworkEvent<B, H>> {
        let (tx_out, rx_out) = mpsc::unbounded();
        let epoch_data = EpochData {
            tx: tx_out,
            status: EpochStatus::InProgress,
            _authorities,
        };
        self.epochs.lock().insert(epoch_id, epoch_data);
        rx_out
    }

    fn sample_random_peer(&self) -> Option<PeerId> {
        self.peers.lock().sample_random()
    }

    fn send_message(&self, peer_id: PeerId, message: Vec<u8>) {
        self.network
            .send_message(peer_id, self.protocol.clone(), message);
    }

    fn on_incoming_message(&self, peer_id: PeerId, raw_message: Vec<u8>) {
        let mut raw_message = raw_message.as_slice();
        match NetworkMessage::<B, H>::decode(&mut raw_message) {
            Ok(NetworkMessage::Consensus(message, epoch_id)) => {
                let mut epochs = self.epochs.lock();
                let maybe_data = epochs.get_mut(&epoch_id);
                if let Some(epoch_data) = maybe_data {
                    if epoch_data.status == EpochStatus::InProgress {
                        if let Err(e) = epoch_data
                            .tx
                            .unbounded_send(NetworkEvent::MessageReceived(message, peer_id))
                        {
                            //TODO: need to write some logic on when an epoch should be terminated and make sure
                            // that there are no issues with synchronization when terminating.
                            epoch_data.status = EpochStatus::Terminated;
                            debug!(target: "network", "Error {:?} when passing a message event to epoch {:?}.", e, epoch_id);
                        }
                    }
                }
            }
            Err(e) => {
                debug!(target: "network", "Error decoding message: {}", e);
            }
        }
    }

    fn on_command(&self, command: NetworkCommand<B, H>) {
        debug!(target: "network", "Received command {:?}", command);
        match command {
            NetworkCommand::SendToAll(message) => {
                debug!(target: "network", "Sending message to {} peers.", self.peers.lock().peers.len());
                for (peer_id, _) in self.peers.lock().iter() {
                    self.send_message(*peer_id, message.encode());
                }
            }
            NetworkCommand::SendToPeer(message, peer_id) => {
                self.send_message(peer_id, message.encode());
            }
            NetworkCommand::SendToRandPeer(message) => {
                if let Some(peer_id) = self.sample_random_peer() {
                    self.send_message(peer_id, message.encode());
                } else {
                    debug!(target: "network", "Attempting to send a message, but no connected peers.");
                }
            }
        }
    }

    fn on_peer_connected(&self, peer_id: PeerId) {
        self.peers.lock().insert(peer_id);
        for (id, epoch_data) in self.epochs.lock().iter() {
            if epoch_data.status == EpochStatus::InProgress {
                if let Err(e) = epoch_data
                    .tx
                    .unbounded_send(NetworkEvent::PeerConnected(peer_id))
                {
                    debug!(target: "network", "Error {:?} when passing connect event to epoch {:?}.", e, id);
                }
            }
        }
    }

    fn on_peer_disconnected(&self, peer_id: PeerId) {
        self.peers.lock().remove(&peer_id);
        for (id, epoch_data) in self.epochs.lock().iter() {
            if epoch_data.status == EpochStatus::InProgress {
                if let Err(e) = epoch_data
                    .tx
                    .unbounded_send(NetworkEvent::PeerDisconnected(peer_id))
                {
                    debug!(target: "network", "Error {:?} when passing disconnect event to epoch {:?}.", e, id);
                }
            }
        }
    }

    pub async fn run(&self, mut net_command_rx: mpsc::UnboundedReceiver<NetworkCommand<B, H>>) {
        let mut network_event_stream = self.network.event_stream();

        loop {
            tokio::select! {
                maybe_event = network_event_stream.next() =>
                     {
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
                                    self.on_peer_disconnected(remote);
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
                maybe_cmd = net_command_rx.next() => {
                    if let Some(cmd) = maybe_cmd {
                        self.on_command(cmd);
                    } else {
                        //TODO: The environment event stream closed, what shall we do?
                        break;
                    }
                }
            }

            self.epochs
                .lock()
                .retain(|_, data| data.status == EpochStatus::InProgress);
        }
    }
}
