use crate::new_network::{Network, NetworkEventStream, NetworkIdentity, PeerId, RequestBlocks};
use async_trait::async_trait;
use log::error;
use sc_network::{multiaddr, ExHashT, Multiaddr, NetworkService, NetworkStateInfo};
use sp_api::NumberFor;
use sp_runtime::traits::Block;
use std::{borrow::Cow, collections::HashSet, fmt, sync::Arc, time::Duration};
use tokio::time::timeout;

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

#[derive(Debug, Copy, Clone)]
pub enum SendError {
    NotConnectedToPeer(PeerId),
    LostConnectionToPeer(PeerId),
    LostConnectionToPeerReady(PeerId),
    SendTimeout(PeerId, u64),
}

const SEND_TO_PEER_TIMEOUT_MS: u64 = 200;

impl fmt::Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SendError::NotConnectedToPeer(peer_id) => {
                write!(f, "Not connected to peer {:?}", peer_id)
            }
            SendError::LostConnectionToPeer(peer_id) => {
                write!(
                    f,
                    "Lost connection to peer {:?} while preparing sender",
                    peer_id
                )
            }
            SendError::LostConnectionToPeerReady(peer_id) => {
                write!(
                    f,
                    "Lost connection to peer {:?} after sender was ready",
                    peer_id
                )
            }
            SendError::SendTimeout(peer_id, timeout) => {
                write!(
                    f,
                    "Timeout while sending to peer {:?} took over {:?} ms",
                    peer_id, timeout
                )
            }
        }
    }
}

impl std::error::Error for SendError {}

#[async_trait]
impl<B: Block, H: ExHashT> Network for Arc<NetworkService<B, H>> {
    type SendError = SendError;

    fn event_stream(&self) -> NetworkEventStream {
        Box::pin(self.as_ref().event_stream("aleph-network"))
    }

    async fn send<'a>(
        &'a self,
        data: impl Into<Vec<u8>> + Send + Sync + 'static,
        peer_id: PeerId,
        protocol: Cow<'static, str>,
    ) -> Result<(), SendError> {
        timeout(
            Duration::from_millis(SEND_TO_PEER_TIMEOUT_MS),
            self.notification_sender(peer_id.into(), protocol)
                .map_err(|_| SendError::NotConnectedToPeer(peer_id))?
                .ready(),
        )
        .await
        .map_err(|_| SendError::SendTimeout(peer_id, SEND_TO_PEER_TIMEOUT_MS))?
        .map_err(|_| SendError::LostConnectionToPeer(peer_id))?
        .send(data)
        .map_err(|_| SendError::LostConnectionToPeerReady(peer_id))
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
