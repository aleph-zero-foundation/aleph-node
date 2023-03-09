use std::collections::{HashMap, HashSet};

use crate::{network::PeerId, SessionId};

/// Keeps track of connections we should maintain taking into account data from many sessions.
pub struct Connections<PID: PeerId> {
    associated_sessions: HashMap<PID, HashSet<SessionId>>,
    peers_by_session: HashMap<SessionId, HashSet<PID>>,
}

impl<PID: PeerId> Connections<PID> {
    /// Creates a new object, initially without any connections.
    pub fn new() -> Self {
        Connections {
            associated_sessions: HashMap::new(),
            peers_by_session: HashMap::new(),
        }
    }

    /// Mark the specified peers as ones we should be connected to for the given session.
    pub fn add_peers(&mut self, session_id: SessionId, peers: impl IntoIterator<Item = PID>) {
        for peer in peers {
            self.associated_sessions
                .entry(peer.clone())
                .or_default()
                .insert(session_id);
            self.peers_by_session
                .entry(session_id)
                .or_default()
                .insert(peer);
        }
    }

    /// Assume we no longer need to be connected to peers from the given session.
    /// Returns the peers we no longer have any reason to be connected to.
    pub fn remove_session(&mut self, session_id: SessionId) -> HashSet<PID> {
        let mut result = HashSet::new();
        if let Some(peers) = self.peers_by_session.remove(&session_id) {
            for peer in peers {
                if let Some(mut sessions) = self.associated_sessions.remove(&peer) {
                    sessions.remove(&session_id);
                    if !sessions.is_empty() {
                        self.associated_sessions.insert(peer, sessions);
                    } else {
                        result.insert(peer);
                    }
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use network_clique::mock::{random_keys, MockPublicKey};

    use super::Connections;
    use crate::SessionId;

    fn random_peer_ids(num: usize) -> HashSet<MockPublicKey> {
        random_keys(num).into_keys().collect()
    }

    #[test]
    fn removes_peer_after_single_session() {
        let session_id = SessionId(43);
        let peer_ids = random_peer_ids(1);
        let mut connections = Connections::new();
        connections.add_peers(session_id, peer_ids.clone());
        let to_remove = connections.remove_session(session_id);
        assert_eq!(to_remove, peer_ids);
    }

    #[test]
    fn does_not_remove_peer_if_still_in_session() {
        let session_id = SessionId(43);
        let other_session_id = SessionId(2137);
        let peer_ids = random_peer_ids(1);
        let mut connections = Connections::new();
        connections.add_peers(session_id, peer_ids.clone());
        connections.add_peers(other_session_id, peer_ids);
        let to_remove = connections.remove_session(session_id);
        assert!(to_remove.is_empty());
    }

    #[test]
    fn removes_peer_only_after_all_sessions_pass() {
        let start = 43;
        let end = 50;
        let peer_ids = random_peer_ids(1);
        let mut connections = Connections::new();
        for i in start..end + 1 {
            connections.add_peers(SessionId(i), peer_ids.clone());
        }
        for i in start..end {
            assert!(connections.remove_session(SessionId(i)).is_empty());
        }
        let to_remove = connections.remove_session(SessionId(end));
        assert_eq!(to_remove, peer_ids);
    }
}
