use std::collections::HashMap;

use sc_network::{ExtendedPeerInfo, PeerId};
use sc_network_common::role::Roles;
use sp_runtime::traits::{Block, Header};

use crate::{BlockHash, BlockNumber};

struct PeerInfo {
    role: Roles,
}

/// Handler for the base protocol.
pub struct Handler {
    peers: HashMap<PeerId, PeerInfo>,
}

impl Handler {
    /// Create a new handler.
    pub fn new() -> Self {
        Handler {
            peers: HashMap::new(),
        }
    }

    /// Returns a list of connected peers with some additional information.
    // TODO(A0-3886): This shouldn't need to return the substrate type after replacing RPCs.
    // In particular, it shouldn't depend on `B`.
    pub fn peers_info<B>(&self) -> Vec<(PeerId, ExtendedPeerInfo<B>)>
    where
        B: Block<Hash = BlockHash>,
        B::Header: Header<Number = BlockNumber>,
    {
        self.peers
            .iter()
            .map(|(id, info)| {
                (
                    *id,
                    ExtendedPeerInfo {
                        roles: info.role,
                        best_hash: Default::default(),
                        best_number: 0,
                    },
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::{aleph_primitives::Block, base_protocol::handler::Handler};

    #[test]
    fn initially_no_peers() {
        let handler = Handler::new();
        assert!(
            handler.peers_info::<Block>().is_empty(),
            "there should be no peers initially"
        );
    }
}
