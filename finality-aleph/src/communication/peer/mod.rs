use rand::{seq::SliceRandom, thread_rng};
use sc_network::{ObservedRole, PeerId};
use std::collections::HashMap;

pub(crate) mod rep;

#[derive(Debug, Clone)]
pub struct PeerInfo {
    role: ObservedRole,
}

impl PeerInfo {
    fn new(role: ObservedRole) -> Self {
        PeerInfo { role }
    }
}

#[derive(Debug, Default)]
pub(crate) struct Peers {
    peers: HashMap<PeerId, PeerInfo>,
}

impl Peers {
    pub(crate) fn insert(&mut self, peer: PeerId, role: ObservedRole) {
        self.peers.insert(peer, PeerInfo::new(role));
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
}
