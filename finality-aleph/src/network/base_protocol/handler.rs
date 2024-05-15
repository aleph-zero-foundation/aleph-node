use std::{
    collections::{HashMap, HashSet},
    fmt::{Display, Formatter, Result as FmtResult},
};

use parity_scale_codec::{DecodeAll, Error as CodecError};
use sc_network::{config::NetworkConfiguration, service::traits::Direction, PeerId};
use sc_network_common::{role::Roles, sync::message::BlockAnnouncesHandshake};
use sc_network_sync::types::ExtendedPeerInfo;
use sp_core::H256;
use sp_runtime::traits::{Block, Header, Saturating};

use crate::{BlockHash, BlockNumber};

/// The role of the connected node.
#[derive(Clone, Copy, Debug)]
pub enum Role {
    /// A full node, the expected type.
    Full,
    /// A light node, we support these connecting to us, but don't provide any implementations.
    Light,
}

impl From<Roles> for Role {
    fn from(roles: Roles) -> Self {
        match roles.is_full() {
            true => Role::Full,
            false => Role::Light,
        }
    }
}

impl From<Role> for Roles {
    fn from(role: Role) -> Self {
        match role {
            Role::Full => Roles::FULL,
            Role::Light => Roles::LIGHT,
        }
    }
}

#[derive(Clone, Debug)]
struct PeerInfo {
    role: Role,
    direction: Direction,
}

impl PeerInfo {
    pub fn new(role: Role, direction: Direction) -> Self {
        PeerInfo { role, direction }
    }
}

// the peer info is never actually inspected, so return dummy values
const DUMMY_HASH: H256 = H256([0; 32]);
const DUMMY_NUMBER: u32 = 0;

impl<B> From<PeerInfo> for ExtendedPeerInfo<B>
where
    B: Block<Hash = BlockHash>,
    B::Header: Header<Number = BlockNumber>,
{
    fn from(peer_info: PeerInfo) -> Self {
        ExtendedPeerInfo {
            roles: peer_info.role.into(),
            best_hash: DUMMY_HASH,
            best_number: DUMMY_NUMBER,
        }
    }
}

/// Reasons to refuse connecting to a peer.
#[derive(Clone, Debug)]
pub enum ConnectError {
    /// We weren't able to decode the handshake.
    BadlyEncodedHandshake(CodecError),
    /// The peer is running on a different chain.
    BadHandshakeGenesis(BlockHash),
    /// The peer is already connected.
    AlreadyConnected(PeerId),
    /// There are too many full peers already connected in the given direction.
    TooManyFullPeers(Direction),
    /// There are too many light peers already connected.
    TooManyLightPeers,
}

impl ConnectError {
    fn too_many_peers(peer: PeerInfo) -> Self {
        use ConnectError::*;
        use Role::*;
        match peer.role {
            Full => TooManyFullPeers(peer.direction),
            Light => TooManyLightPeers,
        }
    }
}

impl Display for ConnectError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        use ConnectError::*;
        match self {
            BadlyEncodedHandshake(e) => write!(f, "failed to decode handshake: {}", e),
            BadHandshakeGenesis(genesis) => write!(f, "peer has different genesis {}", genesis),
            AlreadyConnected(peer_id) => write!(f, "peer {} already connected", peer_id),
            TooManyFullPeers(direction) => {
                write!(f, "too many full nodes connected {:?}", direction)
            }
            TooManyLightPeers => write!(f, "too many light nodes connected"),
        }
    }
}

/// Problems when handling peer disconnecting.
#[derive(Clone, Debug)]
pub enum DisconnectError {
    /// The peer was not connected as far as we know.
    PeerWasNotConnected,
}

impl Display for DisconnectError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        use DisconnectError::*;
        match self {
            PeerWasNotConnected => write!(f, "peer was not connected"),
        }
    }
}

struct ConnectionLimits {
    num_full_in_peers: usize,
    num_full_out_peers: usize,
    num_light_peers: usize,
    max_full_in_peers: usize,
    max_full_out_peers: usize,
    max_light_peers: usize,
}

impl ConnectionLimits {
    pub fn new(net_config: &NetworkConfiguration) -> Self {
        // It is assumed that `default_peers_set.out_peers` only refers to full nodes, but
        // `default_peers_set.in_peers` refers to both full and light nodes.
        // Moreover, `default_peers_set_num_full` refers to the total of full nodes.
        let max_full_out_peers = net_config.default_peers_set.out_peers as usize;
        let max_full_in_peers =
            (net_config.default_peers_set_num_full as usize).saturating_sub(max_full_out_peers);
        let max_light_peers =
            (net_config.default_peers_set.in_peers as usize).saturating_sub(max_full_in_peers);
        ConnectionLimits {
            num_full_in_peers: 0,
            num_full_out_peers: 0,
            num_light_peers: 0,
            max_full_in_peers,
            max_full_out_peers,
            max_light_peers,
        }
    }

    pub fn allowed(&self, peer: &PeerInfo) -> bool {
        match (peer.role, peer.direction) {
            (Role::Light, _) => self.num_light_peers < self.max_light_peers,
            (Role::Full, Direction::Inbound) => self.num_full_in_peers < self.max_full_in_peers,
            (Role::Full, Direction::Outbound) => self.num_full_out_peers < self.max_full_out_peers,
        }
    }

    fn count_for(&mut self, peer: &PeerInfo) -> &mut usize {
        match (peer.role, peer.direction) {
            (Role::Light, _) => &mut self.num_light_peers,
            (Role::Full, Direction::Inbound) => &mut self.num_full_in_peers,
            (Role::Full, Direction::Outbound) => &mut self.num_full_out_peers,
        }
    }

    pub fn add(&mut self, peer: &PeerInfo) {
        self.count_for(peer).saturating_inc();
    }

    pub fn remove(&mut self, peer: &PeerInfo) {
        self.count_for(peer).saturating_dec();
    }
}

/// Handler for the base protocol. Deals with accepting and counting connections.
pub struct Handler<B>
where
    B: Block<Hash = BlockHash>,
    B::Header: Header<Number = BlockNumber>,
{
    reserved_nodes: HashSet<PeerId>,
    peers: HashMap<PeerId, PeerInfo>,
    // the limits ignore the nodes which belong to `reserved_nodes`
    limits: ConnectionLimits,
    genesis_hash: B::Hash,
}

impl<B> Handler<B>
where
    B: Block<Hash = BlockHash>,
    B::Header: Header<Number = BlockNumber>,
{
    /// Create a new handler.
    pub fn new(genesis_hash: B::Hash, net_config: &NetworkConfiguration) -> Self {
        let reserved_nodes = net_config
            .default_peers_set
            .reserved_nodes
            .iter()
            .map(|reserved| reserved.peer_id)
            .collect();
        let limits = ConnectionLimits::new(net_config);

        Handler {
            reserved_nodes,
            peers: HashMap::new(),
            limits,
            genesis_hash,
        }
    }

    fn is_reserved(&self, peer_id: &PeerId) -> bool {
        self.reserved_nodes.contains(peer_id)
    }

    fn verify_connection(
        &self,
        peer_id: PeerId,
        handshake: Vec<u8>,
        direction: Direction,
    ) -> Result<PeerInfo, ConnectError> {
        let handshake = BlockAnnouncesHandshake::<B>::decode_all(&mut &handshake[..])
            .map_err(ConnectError::BadlyEncodedHandshake)?;
        if handshake.genesis_hash != self.genesis_hash {
            return Err(ConnectError::BadHandshakeGenesis(handshake.genesis_hash));
        }

        if self.peers.contains_key(&peer_id) {
            return Err(ConnectError::AlreadyConnected(peer_id));
        }

        let peer = PeerInfo::new(handshake.roles.into(), direction);

        match self.is_reserved(&peer_id) || self.limits.allowed(&peer) {
            true => Ok(peer),
            false => Err(ConnectError::too_many_peers(peer)),
        }
    }

    /// Accept or reject a peer.
    pub fn on_peer_connect(
        &mut self,
        peer_id: PeerId,
        handshake: Vec<u8>,
        direction: Direction,
    ) -> Result<(), ConnectError> {
        let peer = self.verify_connection(peer_id, handshake, direction)?;

        if !self.is_reserved(&peer_id) {
            self.limits.add(&peer);
        }

        self.peers.insert(peer_id, peer);

        Ok(())
    }

    /// Clean up a disconnected peer.
    pub fn on_peer_disconnect(&mut self, peer_id: PeerId) -> Result<(), DisconnectError> {
        let peer = self
            .peers
            .remove(&peer_id)
            .ok_or(DisconnectError::PeerWasNotConnected)?;

        if !self.is_reserved(&peer_id) {
            self.limits.remove(&peer)
        }

        Ok(())
    }

    /// Checks whether an inbound peer would be accepted.
    pub fn verify_inbound_connection(
        &mut self,
        peer_id: PeerId,
        handshake: Vec<u8>,
    ) -> Result<(), ConnectError> {
        self.verify_connection(peer_id, handshake, Direction::Inbound)
            .map(|_| ())
    }

    /// Return information about connected peers. Mostly dummy, although the roles and number of peers match.
    pub fn peers_info(&self) -> Vec<(PeerId, ExtendedPeerInfo<B>)> {
        self.peers
            .iter()
            .map(|(peer_id, peer_info)| (*peer_id, peer_info.clone().into()))
            .collect()
    }
}
