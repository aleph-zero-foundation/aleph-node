use rush::EpochId;
use sc_network::{ObservedRole, PeerId};
use std::collections::HashMap;

pub(crate) mod rep;

#[derive(Debug, Clone)]
pub struct PeerInfo {
    epoch_id: EpochId,
    role: ObservedRole,
}

impl PeerInfo {
    fn new(role: ObservedRole) -> Self {
        PeerInfo {
            epoch_id: EpochId::default(),
            role,
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct Peers {
    authorities: HashMap<PeerId, PeerInfo>,
    others: HashMap<PeerId, PeerInfo>,
}

impl Peers {
    pub(crate) fn insert(&mut self, peer: PeerId, role: ObservedRole) {
        let _ = match role {
            ObservedRole::Authority => self.authorities.insert(peer, PeerInfo::new(role)),
            _ => self.others.insert(peer, PeerInfo::new(role)),
        };
    }

    pub(crate) fn remove(&mut self, peer: &PeerId) {
        self.authorities.remove(peer.as_ref());
        self.others.remove(peer.as_ref());
    }

    pub(crate) fn contains(&self, peer: &PeerId) -> bool {
        self.authorities.contains_key(peer.as_ref()) || self.others.contains_key(peer.as_ref())
    }

    pub(crate) fn contains_authority(&self, peer: &PeerId) -> bool {
        self.authorities.contains_key(peer.as_ref())
    }
}
