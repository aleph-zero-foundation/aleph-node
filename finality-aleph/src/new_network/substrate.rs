use crate::new_network::{Network, NetworkEventStream, NetworkIdentity, PeerId, RequestBlocks};
use log::{debug, error};
use sc_network::{
    multiaddr, ExHashT, Multiaddr, NetworkService, NetworkStateInfo, NotificationSender,
};
use sp_api::NumberFor;
use sp_runtime::traits::Block;
use std::{borrow::Cow, collections::HashSet, sync::Arc};

impl<B: Block, H: ExHashT> RequestBlocks<B> for Arc<NetworkService<B, H>> {
    fn request_justification(&self, hash: &B::Hash, number: NumberFor<B>) {
        NetworkService::request_justification(self, hash, number)
    }

    fn request_stale_block(&self, hash: B::Hash, number: NumberFor<B>) {
        // The below comment is adapted from substrate:
        // Notifies the sync service to try and sync the given block from the given peers. If the given vector
        // of peers is empty (as in our case) then the underlying implementation should make a best effort to fetch
        // the block from any peers it is connected to.
        NetworkService::set_sync_fork_request(self, Vec::new(), hash, number)
    }
}

impl<B: Block, H: ExHashT> Network for Arc<NetworkService<B, H>> {
    fn event_stream(&self) -> NetworkEventStream {
        Box::pin(self.as_ref().event_stream("aleph-network"))
    }

    fn message_sender(
        &self,
        peer_id: PeerId,
        protocol: Cow<'static, str>,
    ) -> Option<NotificationSender> {
        self.notification_sender(peer_id.into(), protocol)
            .map_err(|_| debug!(target: "aleph-network", "Attempted send to peer we are not connected to."))
            .ok()
    }

    fn add_reserved(&self, addresses: HashSet<Multiaddr>, protocol: Cow<'static, str>) {
        let result = self.add_peers_to_reserved_set(protocol, addresses);
        if let Err(e) = result {
            error!(target: "aleph-network", "add_reserved failed: {}", e);
        }
    }

    fn remove_reserved(&self, peers: HashSet<PeerId>, protocol: Cow<'static, str>) {
        let addresses = peers
            .into_iter()
            .map(|peer_id| Multiaddr::empty().with(multiaddr::Protocol::P2p(peer_id.0.into())))
            .collect();
        let result = self.remove_peers_from_reserved_set(protocol, addresses);
        if let Err(e) = result {
            error!(target: "aleph-network", "remove_reserved failed: {}", e);
        }
    }
}

impl<B: Block, H: ExHashT> NetworkIdentity for Arc<NetworkService<B, H>> {
    fn identity(&self) -> (Vec<Multiaddr>, PeerId) {
        (self.external_addresses(), (*self.local_peer_id()).into())
    }
}
