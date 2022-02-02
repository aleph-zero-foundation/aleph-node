use crate::network::{
    Network, NetworkEventStream, NetworkIdentity, NetworkSender, PeerId, RequestBlocks,
};
use async_trait::async_trait;
use log::error;
use sc_network::{ExHashT, Multiaddr, NetworkService, NetworkStateInfo, NotificationSender};
use sp_api::NumberFor;
use sp_runtime::traits::Block;
use std::{borrow::Cow, collections::HashSet, fmt, sync::Arc};

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

    /// Clear all pending justification requests.
    fn clear_justification_requests(&self) {
        NetworkService::clear_justification_requests(self)
    }
}

#[derive(Debug)]
pub enum SenderError {
    CannotCreateSender(PeerId, Cow<'static, str>),
    LostConnectionToPeer(PeerId),
    LostConnectionToPeerReady(PeerId),
}

impl fmt::Display for SenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SenderError::CannotCreateSender(peer_id, protocol) => {
                write!(
                    f,
                    "Can not create sender to peer {:?} with protocol {:?}",
                    peer_id, protocol
                )
            }
            SenderError::LostConnectionToPeer(peer_id) => {
                write!(
                    f,
                    "Lost connection to peer {:?} while preparing sender",
                    peer_id
                )
            }
            SenderError::LostConnectionToPeerReady(peer_id) => {
                write!(
                    f,
                    "Lost connection to peer {:?} after sender was ready",
                    peer_id
                )
            }
        }
    }
}

impl std::error::Error for SenderError {}

pub struct SubstrateNetworkSender {
    notification_sender: NotificationSender,
    peer_id: PeerId,
}

#[async_trait]
impl NetworkSender for SubstrateNetworkSender {
    type SenderError = SenderError;

    async fn send<'a>(
        &'a self,
        data: impl Into<Vec<u8>> + Send + Sync + 'static,
    ) -> Result<(), SenderError> {
        self.notification_sender
            .ready()
            .await
            .map_err(|_| SenderError::LostConnectionToPeer(self.peer_id))?
            .send(data)
            .map_err(|_| SenderError::LostConnectionToPeerReady(self.peer_id))
    }
}

impl<B: Block, H: ExHashT> Network for Arc<NetworkService<B, H>> {
    type SenderError = SenderError;
    type NetworkSender = SubstrateNetworkSender;

    fn event_stream(&self) -> NetworkEventStream {
        Box::pin(self.as_ref().event_stream("aleph-network"))
    }

    fn sender(
        &self,
        peer_id: PeerId,
        protocol: Cow<'static, str>,
    ) -> Result<Self::NetworkSender, Self::SenderError> {
        Ok(SubstrateNetworkSender {
            // Currently method `notification_sender` does not distinguish whether we are not connected to the peer
            // or there is no such protocol so we need to have this worthless `SenderError::CannotCreateSender` error here
            notification_sender: self
                .notification_sender(peer_id.into(), protocol.clone())
                .map_err(|_| SenderError::CannotCreateSender(peer_id, protocol))?,
            peer_id,
        })
    }

    fn add_reserved(&self, addresses: HashSet<Multiaddr>, protocol: Cow<'static, str>) {
        let result = self.add_peers_to_reserved_set(protocol, addresses);
        if let Err(e) = result {
            error!(target: "aleph-network", "add_reserved failed: {}", e);
        }
    }

    fn remove_reserved(&self, peers: HashSet<PeerId>, protocol: Cow<'static, str>) {
        let addresses = peers.into_iter().map(|peer_id| peer_id.0).collect();
        self.remove_peers_from_reserved_set(protocol, addresses);
    }
}

impl<B: Block, H: ExHashT> NetworkIdentity for Arc<NetworkService<B, H>> {
    fn identity(&self) -> (Vec<Multiaddr>, PeerId) {
        (self.external_addresses(), (*self.local_peer_id()).into())
    }
}
