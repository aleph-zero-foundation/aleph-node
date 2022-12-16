use std::{
    collections::{HashMap, HashSet},
    fmt::{Display, Error as FmtError, Formatter},
};

use futures::channel::mpsc;

use crate::network::{
    clique::{
        manager::{AddResult, SendError},
        PublicKey,
    },
    Data, PeerId,
};

/// Network component responsible for holding the list of peers that we
/// want to connect to, and managing the established connections.
pub struct Manager<PK: PublicKey + PeerId, A: Data, D: Data> {
    addresses: HashMap<PK, A>,
    outgoing: HashMap<PK, mpsc::UnboundedSender<D>>,
    incoming: HashMap<PK, mpsc::UnboundedSender<D>>,
}

struct ManagerStatus<PK: PublicKey + PeerId> {
    wanted_peers: usize,
    both_ways_peers: HashSet<PK>,
    outgoing_peers: HashSet<PK>,
    incoming_peers: HashSet<PK>,
    missing_peers: HashSet<PK>,
}

impl<PK: PublicKey + PeerId> ManagerStatus<PK> {
    fn new<A: Data, D: Data>(manager: &Manager<PK, A, D>) -> Self {
        let incoming: HashSet<_> = manager
            .incoming
            .iter()
            .filter(|(_, exit)| !exit.is_closed())
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

impl<PK: PublicKey + PeerId> Display for ManagerStatus<PK> {
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
                .map(|peer_id| peer_id.to_short_string())
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
                .map(|peer_id| peer_id.to_short_string())
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
                .map(|peer_id| peer_id.to_short_string())
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
                .map(|peer_id| peer_id.to_short_string())
                .collect::<Vec<_>>()
                .join(", ");
            write!(f, "missing - {:?} [{}];", self.missing_peers.len(), peers)?;
        }

        Ok(())
    }
}

impl<PK: PublicKey + PeerId, A: Data, D: Data> Manager<PK, A, D> {
    /// Create a new Manager with empty list of peers.
    pub fn new() -> Self {
        Manager {
            addresses: HashMap::new(),
            outgoing: HashMap::new(),
            incoming: HashMap::new(),
        }
    }

    /// Add a peer to the list of peers we want to stay connected to, or
    /// update the address if the peer was already added.
    /// Returns whether this peer is a new peer.
    pub fn add_peer(&mut self, peer_id: PK, address: A) -> bool {
        self.addresses.insert(peer_id, address).is_none()
    }

    /// Return Option containing the address of the given peer, or None if
    /// the peer is unknown.
    pub fn peer_address(&self, peer_id: &PK) -> Option<A> {
        self.addresses.get(peer_id).cloned()
    }

    /// Add an established outgoing connection with a known peer,
    /// but only if the peer is on the list of peers that we want to stay connected with.
    pub fn add_outgoing(
        &mut self,
        peer_id: PK,
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
    pub fn add_incoming(&mut self, peer_id: PK, exit: mpsc::UnboundedSender<D>) -> AddResult {
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
    pub fn remove_peer(&mut self, peer_id: &PK) {
        self.addresses.remove(peer_id);
        self.incoming.remove(peer_id);
        self.outgoing.remove(peer_id);
    }

    /// Send data to a peer.
    /// Returns error if there is no outgoing connection to the peer,
    /// or if the connection is dead.
    pub fn send_to(&mut self, peer_id: &PK, data: D) -> Result<(), SendError> {
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
    use futures::{channel::mpsc, StreamExt};

    use super::{AddResult::*, Manager, SendError};
    use crate::network::clique::mock::{key, MockPublicKey};

    type Data = String;
    type Address = String;

    #[test]
    fn add_remove() {
        let mut manager = Manager::<MockPublicKey, Address, Data>::new();
        let (peer_id, _) = key();
        let (peer_id_b, _) = key();
        let address = String::from("43.43.43.43:43000");
        // add new peer - returns true
        assert!(manager.add_peer(peer_id.clone(), address.clone()));
        // add known peer - returns false
        assert!(!manager.add_peer(peer_id.clone(), address.clone()));
        // get address
        assert_eq!(manager.peer_address(&peer_id), Some(address));
        // try to get address of an unknown peer
        assert_eq!(manager.peer_address(&peer_id_b), None);
        // remove peer
        manager.remove_peer(&peer_id);
        // try to get address of removed peer
        assert_eq!(manager.peer_address(&peer_id), None);
    }

    #[tokio::test]
    async fn outgoing() {
        let mut manager = Manager::<MockPublicKey, Address, Data>::new();
        let data = String::from("DATA");
        let (peer_id, _) = key();
        let address = String::from("43.43.43.43:43000");
        let (tx, _rx) = mpsc::unbounded();
        // try add unknown peer
        manager.add_outgoing(peer_id.clone(), tx);
        // sending should fail
        assert_eq!(
            manager.send_to(&peer_id, data.clone()),
            Err(SendError::PeerNotFound)
        );
        // add peer, this time for real
        assert!(manager.add_peer(peer_id.clone(), address.clone()));
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

    #[test]
    fn incoming() {
        let mut manager = Manager::<MockPublicKey, Address, Data>::new();
        let (peer_id, _) = key();
        let address = String::from("43.43.43.43:43000");
        let (tx, mut rx) = mpsc::unbounded();
        // try add unknown peer
        assert_eq!(manager.add_incoming(peer_id.clone(), tx), Uninterested);
        // rx should fail
        assert!(rx.try_next().expect("channel should be closed").is_none());
        // add peer, this time for real
        assert!(manager.add_peer(peer_id.clone(), address));
        let (tx, mut rx) = mpsc::unbounded();
        // should just add
        assert_eq!(manager.add_incoming(peer_id.clone(), tx), Added);
        // the exit channel should be open
        assert!(rx.try_next().is_err());
        let (tx, mut rx2) = mpsc::unbounded();
        // should replace now
        assert_eq!(manager.add_incoming(peer_id.clone(), tx), Replaced);
        // receiving should fail on old, but work on new channel
        assert!(rx.try_next().expect("channel should be closed").is_none());
        assert!(rx2.try_next().is_err());
        // remove peer
        manager.remove_peer(&peer_id);
        // receiving should fail
        assert!(rx2.try_next().expect("channel should be closed").is_none());
    }
}
