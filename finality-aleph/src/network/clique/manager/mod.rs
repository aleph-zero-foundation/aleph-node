use std::{
    collections::{HashMap, HashSet},
    fmt::{Display, Error as FmtError, Formatter},
};

use futures::channel::mpsc;

use crate::network::{clique::PublicKey, Data, PeerId};

mod direction;
use direction::DirectedPeers;

/// Error during sending data through the Manager
#[derive(Debug, PartialEq, Eq)]
pub enum SendError {
    /// Outgoing network connection closed
    ConnectionClosed,
    /// Peer not added to the manager
    PeerNotFound,
}

impl Display for SendError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use SendError::*;
        match self {
            ConnectionClosed => write!(f, "worker dead"),
            PeerNotFound => write!(f, "peer not found"),
        }
    }
}

/// Possible results of adding connections.
#[derive(Debug, PartialEq, Eq)]
pub enum AddResult {
    /// We do not want to maintain a connection with this peer.
    Uninterested,
    /// Connection added.
    Added,
    /// Old connection replaced with new one.
    Replaced,
}

struct ManagerStatus<PK: PublicKey + PeerId> {
    outgoing_peers: HashSet<PK>,
    missing_outgoing: HashSet<PK>,
    incoming_peers: HashSet<PK>,
    missing_incoming: HashSet<PK>,
}

impl<PK: PublicKey + PeerId> ManagerStatus<PK> {
    fn new<A: Data, D: Data>(manager: &Manager<PK, A, D>) -> Self {
        let mut incoming_peers = HashSet::new();
        let mut missing_incoming = HashSet::new();
        let mut outgoing_peers = HashSet::new();
        let mut missing_outgoing = HashSet::new();

        for peer in manager.wanted.incoming_peers() {
            match manager.active_connection(peer) {
                true => incoming_peers.insert(peer.clone()),
                false => missing_incoming.insert(peer.clone()),
            };
        }
        for peer in manager.wanted.outgoing_peers() {
            match manager.active_connection(peer) {
                true => outgoing_peers.insert(peer.clone()),
                false => missing_outgoing.insert(peer.clone()),
            };
        }
        ManagerStatus {
            incoming_peers,
            missing_incoming,
            outgoing_peers,
            missing_outgoing,
        }
    }

    fn wanted_incoming(&self) -> usize {
        self.incoming_peers.len() + self.missing_incoming.len()
    }

    fn wanted_outgoing(&self) -> usize {
        self.outgoing_peers.len() + self.missing_outgoing.len()
    }
}

fn pretty_peer_id_set<PK: PublicKey + PeerId>(set: &HashSet<PK>) -> String {
    set.iter()
        .map(|peer_id| peer_id.to_short_string())
        .collect::<Vec<_>>()
        .join(", ")
}

impl<PK: PublicKey + PeerId> Display for ManagerStatus<PK> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        let wanted_incoming = self.wanted_incoming();
        let wanted_outgoing = self.wanted_outgoing();
        if wanted_incoming + wanted_outgoing == 0 {
            return write!(f, "not maintaining any connections; ");
        }

        match wanted_incoming {
            0 => write!(f, "not expecting any incoming connections; ")?,
            _ => {
                write!(f, "expecting {:?} incoming connections; ", wanted_incoming)?;
                match self.incoming_peers.is_empty() {
                    // We warn about the lack of incoming connections, because this is relatively
                    // likely to be a common misconfiguration; much less the case for outgoing.
                    true => write!(f, "WARNING! No incoming peers even though we expected them, maybe connecting to us is impossible; ")?,
                    false => write!(
                            f,
                            "have - {:?} [{}]; ",
                            self.incoming_peers.len(),
                            pretty_peer_id_set(&self.incoming_peers),
                    )?,
                }
                if !self.missing_incoming.is_empty() {
                    write!(
                        f,
                        "missing - {:?} [{}]; ",
                        self.missing_incoming.len(),
                        pretty_peer_id_set(&self.missing_incoming),
                    )?;
                }
            }
        }

        match wanted_outgoing {
            0 => write!(f, "not attempting any outgoing connections; ")?,
            _ => {
                write!(f, "attempting {:?} outgoing connections; ", wanted_outgoing)?;
                if !self.outgoing_peers.is_empty() {
                    write!(
                        f,
                        "have - {:?} [{}]; ",
                        self.outgoing_peers.len(),
                        pretty_peer_id_set(&self.outgoing_peers),
                    )?;
                }
                if !self.missing_outgoing.is_empty() {
                    write!(
                        f,
                        "missing - {:?} [{}]; ",
                        self.missing_outgoing.len(),
                        pretty_peer_id_set(&self.missing_outgoing),
                    )?;
                }
            }
        }

        Ok(())
    }
}

/// Network component responsible for holding the list of peers that we
/// want to connect to or let them connect to us, and managing the established
/// connections.
pub struct Manager<PK: PublicKey + PeerId, A: Data, D: Data> {
    // Which peers we want to be connected with, and which way.
    wanted: DirectedPeers<PK, A>,
    // This peers we are connected with. We ensure that this is always a subset of what we want.
    have: HashMap<PK, mpsc::UnboundedSender<D>>,
}

impl<PK: PublicKey + PeerId, A: Data, D: Data> Manager<PK, A, D> {
    /// Create a new Manager with empty list of peers.
    pub fn new(own_id: PK) -> Self {
        Manager {
            wanted: DirectedPeers::new(own_id),
            have: HashMap::new(),
        }
    }

    fn active_connection(&self, peer_id: &PK) -> bool {
        self.have
            .get(peer_id)
            .map(|sender| !sender.is_closed())
            .unwrap_or(false)
    }

    /// Add a peer to the list of peers we want to stay connected to, or
    /// update the address if the peer was already added.
    /// Returns whether we should start attempts at connecting with the peer, which depends on the
    /// coorddinated pseudorandom decision on the direction of the connection and whether this was
    /// added for the first time.
    pub fn add_peer(&mut self, peer_id: PK, address: A) -> bool {
        self.wanted.add_peer(peer_id, address)
    }

    /// Return the address of the given peer, or None if we shouldn't attempt connecting with the peer.
    pub fn peer_address(&self, peer_id: &PK) -> Option<A> {
        self.wanted.peer_address(peer_id)
    }

    /// Add an established connection with a known peer, but only if the peer is among the peers we want to be connected to.
    pub fn add_connection(
        &mut self,
        peer_id: PK,
        data_for_network: mpsc::UnboundedSender<D>,
    ) -> AddResult {
        use AddResult::*;
        if !self.wanted.interested(&peer_id) {
            return Uninterested;
        }
        match self.have.insert(peer_id, data_for_network) {
            Some(_) => Replaced,
            None => Added,
        }
    }

    /// Remove a peer from the list of peers that we want to stay connected with.
    /// Close any incoming and outgoing connections that were established.
    pub fn remove_peer(&mut self, peer_id: &PK) {
        self.wanted.remove_peer(peer_id);
        self.have.remove(peer_id);
    }

    /// Send data to a peer.
    /// Returns error if there is no outgoing connection to the peer,
    /// or if the connection is dead.
    pub fn send_to(&mut self, peer_id: &PK, data: D) -> Result<(), SendError> {
        self.have
            .get(peer_id)
            .ok_or(SendError::PeerNotFound)?
            .unbounded_send(data)
            .map_err(|_| SendError::ConnectionClosed)
    }

    /// A status of the manager, to be displayed somewhere.
    pub fn status_report(&self) -> impl Display {
        ManagerStatus::new(self)
    }

    pub fn is_authorized(&self, public_key: &PK) -> bool {
        self.wanted.interested(public_key)
    }
}

#[cfg(test)]
mod tests {
    use futures::{channel::mpsc, StreamExt};

    use super::{AddResult::*, Manager, SendError};
    use crate::network::clique::mock::{key, MockPublicKey};

    type Data = String;
    type Address = String;

    #[test]
    fn add_remove() {
        let (own_id, _) = key();
        let mut manager = Manager::<MockPublicKey, Address, Data>::new(own_id);
        let (peer_id, _) = key();
        let (peer_id_b, _) = key();
        let address = String::from("43.43.43.43:43000");
        // add new peer - might return either true or false, depending on the ids
        let attempting_connections = manager.add_peer(peer_id.clone(), address.clone());
        // add known peer - always returns false
        assert!(!manager.add_peer(peer_id.clone(), address.clone()));
        // get address
        match attempting_connections {
            true => assert_eq!(manager.peer_address(&peer_id), Some(address)),
            false => assert_eq!(manager.peer_address(&peer_id), None),
        }
        // try to get address of an unknown peer
        assert_eq!(manager.peer_address(&peer_id_b), None);
        // remove peer
        manager.remove_peer(&peer_id);
        // try to get address of removed peer
        assert_eq!(manager.peer_address(&peer_id), None);
        // remove again
        manager.remove_peer(&peer_id);
        // remove unknown peer
        manager.remove_peer(&peer_id_b);
    }

    #[tokio::test]
    async fn send_receive() {
        let (mut connecting_id, _) = key();
        let mut connecting_manager =
            Manager::<MockPublicKey, Address, Data>::new(connecting_id.clone());
        let (mut listening_id, _) = key();
        let mut listening_manager =
            Manager::<MockPublicKey, Address, Data>::new(listening_id.clone());
        let data = String::from("DATA");
        let address = String::from("43.43.43.43:43000");
        let (tx, _rx) = mpsc::unbounded();
        // try add unknown peer
        assert_eq!(
            connecting_manager.add_connection(listening_id.clone(), tx),
            Uninterested
        );
        // sending should fail
        assert_eq!(
            connecting_manager.send_to(&listening_id, data.clone()),
            Err(SendError::PeerNotFound)
        );
        // add peer, this time for real
        if connecting_manager.add_peer(listening_id.clone(), address.clone()) {
            assert!(!listening_manager.add_peer(connecting_id.clone(), address.clone()))
        } else {
            // We need to switch the names around, because the connection was randomly the
            // other way around.
            std::mem::swap(&mut connecting_id, &mut listening_id);
            std::mem::swap(&mut connecting_manager, &mut listening_manager);
            assert!(connecting_manager.add_peer(listening_id.clone(), address.clone()));
        }
        // add outgoing to connecting
        let (tx, mut rx) = mpsc::unbounded();
        assert_eq!(
            connecting_manager.add_connection(listening_id.clone(), tx),
            Added
        );
        // send and receive connecting
        assert!(connecting_manager
            .send_to(&listening_id, data.clone())
            .is_ok());
        assert_eq!(data, rx.next().await.expect("should receive"));
        // add incoming to listening
        let (tx, mut rx) = mpsc::unbounded();
        assert_eq!(
            listening_manager.add_connection(connecting_id.clone(), tx),
            Added
        );
        // send and receive listening
        assert!(listening_manager
            .send_to(&connecting_id, data.clone())
            .is_ok());
        assert_eq!(data, rx.next().await.expect("should receive"));
        // remove peer
        listening_manager.remove_peer(&connecting_id);
        // receiving should fail
        assert!(rx.next().await.is_none());
    }
}
