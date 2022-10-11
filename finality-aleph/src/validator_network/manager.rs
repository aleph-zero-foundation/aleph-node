use std::{
    collections::HashMap,
    fmt::{Display, Error as FmtError, Formatter},
};

use aleph_primitives::AuthorityId;
use futures::channel::{mpsc, oneshot};

use crate::validator_network::Data;

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
    incoming_peers: usize,
    outgoing_peers: usize,
}

impl Display for ManagerStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        if self.wanted_peers == 0 {
            return write!(f, "not maintaining any connections");
        }
        if self.incoming_peers == 0 {
            write!(f, "WARNING! No incoming peers even though we expected tham, maybe connecting to us is impossible.")?;
        }
        write!(
            f,
            "maintaining {} connections, incoming connections {}, outgoing connections {}",
            self.wanted_peers, self.incoming_peers, self.outgoing_peers,
        )
    }
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
    ) {
        if self.addresses.contains_key(&peer_id) {
            self.outgoing.insert(peer_id, data_for_network);
        };
    }

    /// Add an established incoming connection with a known peer,
    /// but only if the peer is on the list of peers that we want to stay connected with.
    /// Returns true if it overwrote an earlier connection.
    pub fn add_incoming(&mut self, peer_id: AuthorityId, exit: oneshot::Sender<()>) -> bool {
        if !self.addresses.contains_key(&peer_id) {
            return false;
        };
        self.incoming.insert(peer_id, exit).is_some()
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
        ManagerStatus {
            wanted_peers: self.addresses.len(),
            incoming_peers: self
                .incoming
                .values()
                .filter(|exit| exit.is_canceled())
                .count(),
            outgoing_peers: self
                .outgoing
                .values()
                .filter(|sender| sender.is_closed())
                .count(),
        }
    }
}

#[cfg(test)]
mod tests {
    use futures::{
        channel::{mpsc, oneshot},
        StreamExt,
    };

    use super::{Manager, SendError};
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
        manager.add_outgoing(peer_id.clone(), tx);
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
        // try add unknown peer, does not replace so should be false
        assert!(!manager.add_incoming(peer_id.clone(), tx));
        // rx should fail
        assert!(rx.await.is_err());
        // add peer, this time for real
        assert!(manager.add_peer(peer_id.clone(), addresses.clone()));
        let (tx, mut rx) = oneshot::channel();
        // still shouldn't replace
        assert!(!manager.add_incoming(peer_id.clone(), tx));
        // the exit channel should be open
        assert!(rx.try_recv().is_ok());
        let (tx, mut rx2) = oneshot::channel();
        // should replace now
        assert!(manager.add_incoming(peer_id.clone(), tx));
        // receiving should fail on old, but work on new channel
        assert!(rx.try_recv().is_err());
        assert!(rx2.try_recv().is_ok());
        // remove peer
        manager.remove_peer(&peer_id);
        // receiving should fail
        assert!(rx2.try_recv().is_err());
    }
}
