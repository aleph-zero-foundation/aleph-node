use std::{
    collections::{HashMap, HashSet},
    fmt::{Display, Error as FmtError, Formatter},
};

use aleph_primitives::AuthorityId;
use futures::channel::{mpsc, oneshot};

use crate::{network::PeerId, validator_network::Data};

/// Network component responsible for holding the list of peers that we
/// want to connect to, and managing the established connections.
pub struct Manager<A: Data, D: Data> {
    addresses: HashMap<AuthorityId, Vec<A>>,
    outgoing: HashMap<AuthorityId, mpsc::UnboundedSender<D>>,
    incoming: HashMap<AuthorityId, oneshot::Sender<()>>,
}

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

struct ManagerStatus {
    wanted_peers: usize,
    both_ways_peers: HashSet<AuthorityId>,
    outgoing_peers: HashSet<AuthorityId>,
    incoming_peers: HashSet<AuthorityId>,
    missing_peers: HashSet<AuthorityId>,
}

impl ManagerStatus {
    fn new<A: Data, D: Data>(manager: &Manager<A, D>) -> Self {
        let incoming: HashSet<_> = manager
            .incoming
            .iter()
            .filter(|(_, exit)| !exit.is_canceled())
            .map(|(k, _)| k.clone())
            .collect();
        let outgoing: HashSet<_> = manager
            .outgoing
            .iter()
            .filter(|(_, exit)| !exit.is_closed())
            .map(|(k, _)| k.clone())
            .collect();

        let both_ways = incoming.intersection(&outgoing).cloned().collect();
        let incoming: HashSet<_> = incoming.difference(&both_ways).cloned().collect();
        let outgoing: HashSet<_> = outgoing.difference(&both_ways).cloned().collect();
        let missing = manager
            .addresses
            .keys()
            .filter(|a| !both_ways.contains(a) && !incoming.contains(a) && !outgoing.contains(a))
            .cloned()
            .collect();

        ManagerStatus {
            wanted_peers: manager.addresses.len(),
            both_ways_peers: both_ways,
            incoming_peers: incoming,
            outgoing_peers: outgoing,
            missing_peers: missing,
        }
    }
}

impl Display for ManagerStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        if self.wanted_peers == 0 {
            return write!(f, "not maintaining any connections; ");
        }

        write!(f, "target - {:?} connections; ", self.wanted_peers)?;

        if self.both_ways_peers.is_empty() && self.incoming_peers.is_empty() {
            write!(f, "WARNING! No incoming peers even though we expected tham, maybe connecting to us is impossible; ")?;
        }

        if !self.both_ways_peers.is_empty() {
            let peers = self
                .both_ways_peers
                .iter()
                .map(|authority_id| authority_id.to_short_string())
                .collect::<Vec<_>>()
                .join(", ");
            write!(
                f,
                "both ways - {:?} [{}]; ",
                self.both_ways_peers.len(),
                peers,
            )?;
        }

        if !self.incoming_peers.is_empty() {
            let peers = self
                .incoming_peers
                .iter()
                .map(|authority_id| authority_id.to_short_string())
                .collect::<Vec<_>>()
                .join(", ");
            write!(
                f,
                "incoming only - {:?} [{}]; ",
                self.incoming_peers.len(),
                peers
            )?;
        }

        if !self.outgoing_peers.is_empty() {
            let peers = self
                .outgoing_peers
                .iter()
                .map(|authority_id| authority_id.to_short_string())
                .collect::<Vec<_>>()
                .join(", ");
            write!(
                f,
                "outgoing only - {:?} [{}];",
                self.outgoing_peers.len(),
                peers
            )?;
        }

        if !self.missing_peers.is_empty() {
            let peers = self
                .missing_peers
                .iter()
                .map(|authority_id| authority_id.to_short_string())
                .collect::<Vec<_>>()
                .join(", ");
            write!(f, "missing - {:?} [{}];", self.missing_peers.len(), peers)?;
        }

        Ok(())
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

impl<A: Data, D: Data> Manager<A, D> {
    /// Create a new Manager with empty list of peers.
    pub fn new() -> Self {
        Manager {
            addresses: HashMap::new(),
            outgoing: HashMap::new(),
            incoming: HashMap::new(),
        }
    }

    /// Add a peer to the list of peers we want to stay connected to, or
    /// update the list of addresses if the peer was already added.
    /// Returns whether this peer is a new peer.
    pub fn add_peer(&mut self, peer_id: AuthorityId, addresses: Vec<A>) -> bool {
        self.addresses.insert(peer_id, addresses).is_none()
    }

    /// Return Option containing addresses of the given peer, or None if
    /// the peer is unknown.
    pub fn peer_addresses(&self, peer_id: &AuthorityId) -> Option<Vec<A>> {
        self.addresses.get(peer_id).cloned()
    }

    /// Add an established outgoing connection with a known peer,
    /// but only if the peer is on the list of peers that we want to stay connected with.
    pub fn add_outgoing(
        &mut self,
        peer_id: AuthorityId,
        data_for_network: mpsc::UnboundedSender<D>,
    ) -> AddResult {
        use AddResult::*;
        if !self.addresses.contains_key(&peer_id) {
            return Uninterested;
        }
        match self.outgoing.insert(peer_id, data_for_network) {
            Some(_) => Replaced,
            None => Added,
        }
    }

    /// Add an established incoming connection with a known peer,
    /// but only if the peer is on the list of peers that we want to stay connected with.
    pub fn add_incoming(&mut self, peer_id: AuthorityId, exit: oneshot::Sender<()>) -> AddResult {
        use AddResult::*;
        if !self.addresses.contains_key(&peer_id) {
            return Uninterested;
        };
        match self.incoming.insert(peer_id, exit) {
            Some(_) => Replaced,
            None => Added,
        }
    }

    /// Remove a peer from the list of peers that we want to stay connected with.
    /// Close any incoming and outgoing connections that were established.
    pub fn remove_peer(&mut self, peer_id: &AuthorityId) {
        self.addresses.remove(peer_id);
        self.incoming.remove(peer_id);
        self.outgoing.remove(peer_id);
    }

    /// Send data to a peer.
    /// Returns error if there is no outgoing connection to the peer,
    /// or if the connection is dead.
    pub fn send_to(&mut self, peer_id: &AuthorityId, data: D) -> Result<(), SendError> {
        self.outgoing
            .get(peer_id)
            .ok_or(SendError::PeerNotFound)?
            .unbounded_send(data)
            .map_err(|_| SendError::ConnectionClosed)
    }

    /// A status of the manager, to be displayed somewhere.
    pub fn status_report(&self) -> impl Display {
        ManagerStatus::new(self)
    }
}

#[cfg(test)]
mod tests {
    use futures::{
        channel::{mpsc, oneshot},
        StreamExt,
    };

    use super::{AddResult::*, Manager, SendError};
    use crate::validator_network::mock::keys;

    type Data = String;
    type Address = String;

    #[tokio::test]
    async fn add_remove() {
        let mut manager = Manager::<Address, Data>::new();
        let (peer_id, _) = keys().await;
        let (peer_id_b, _) = keys().await;
        let addresses = vec![
            String::from(""),
            String::from("a/b/c"),
            String::from("43.43.43.43:43000"),
        ];
        // add new peer - returns true
        assert!(manager.add_peer(peer_id.clone(), addresses.clone()));
        // add known peer - returns false
        assert!(!manager.add_peer(peer_id.clone(), addresses.clone()));
        // get address
        assert_eq!(manager.peer_addresses(&peer_id), Some(addresses));
        // try to get address of an unknown peer
        assert_eq!(manager.peer_addresses(&peer_id_b), None);
        // remove peer
        manager.remove_peer(&peer_id);
        // try to get address of removed peer
        assert_eq!(manager.peer_addresses(&peer_id), None);
        // remove again
        manager.remove_peer(&peer_id);
        // remove unknown peer
        manager.remove_peer(&peer_id_b);
    }

    #[tokio::test]
    async fn outgoing() {
        let mut manager = Manager::<Address, Data>::new();
        let data = String::from("DATA");
        let (peer_id, _) = keys().await;
        let addresses = vec![
            String::from(""),
            String::from("a/b/c"),
            String::from("43.43.43.43:43000"),
        ];
        let (tx, _rx) = mpsc::unbounded();
        // try add unknown peer
        manager.add_outgoing(peer_id.clone(), tx);
        // sending should fail
        assert_eq!(
            manager.send_to(&peer_id, data.clone()),
            Err(SendError::PeerNotFound)
        );
        // add peer, this time for real
        assert!(manager.add_peer(peer_id.clone(), addresses.clone()));
        let (tx, mut rx) = mpsc::unbounded();
        assert_eq!(manager.add_outgoing(peer_id.clone(), tx), Added);
        // send and receive
        assert!(manager.send_to(&peer_id, data.clone()).is_ok());
        assert_eq!(data, rx.next().await.expect("should receive"));
        // remove peer
        manager.remove_peer(&peer_id);
        // receiving should fail
        assert!(rx.next().await.is_none());
    }

    #[tokio::test]
    async fn incoming() {
        let mut manager = Manager::<Address, Data>::new();
        let (peer_id, _) = keys().await;
        let addresses = vec![
            String::from(""),
            String::from("a/b/c"),
            String::from("43.43.43.43:43000"),
        ];
        let (tx, rx) = oneshot::channel();
        // try add unknown peer
        assert_eq!(manager.add_incoming(peer_id.clone(), tx), Uninterested);
        // rx should fail
        assert!(rx.await.is_err());
        // add peer, this time for real
        assert!(manager.add_peer(peer_id.clone(), addresses.clone()));
        let (tx, mut rx) = oneshot::channel();
        // should just add
        assert_eq!(manager.add_incoming(peer_id.clone(), tx), Added);
        // the exit channel should be open
        assert!(rx.try_recv().is_ok());
        let (tx, mut rx2) = oneshot::channel();
        // should replace now
        assert_eq!(manager.add_incoming(peer_id.clone(), tx), Replaced);
        // receiving should fail on old, but work on new channel
        assert!(rx.try_recv().is_err());
        assert!(rx2.try_recv().is_ok());
        // remove peer
        manager.remove_peer(&peer_id);
        // receiving should fail
        assert!(rx2.try_recv().is_err());
    }
}
