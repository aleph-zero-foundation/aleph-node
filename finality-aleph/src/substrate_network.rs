use std::{borrow::Cow, collections::HashSet, fmt, iter, pin::Pin, sync::Arc};

use async_trait::async_trait;
use futures::stream::{Stream, StreamExt};
use log::error;
use sc_consensus::JustificationSyncLink;
use sc_network::{
    multiaddr::Protocol as MultiaddressProtocol, Event as SubstrateEvent, ExHashT, Multiaddr,
    NetworkService, NetworkSyncForkRequest, PeerId,
};
use sc_network_common::service::{
    NetworkEventStream as _, NetworkNotification, NetworkPeers, NotificationSender,
};
use sp_api::NumberFor;
use sp_consensus::SyncOracle;
use sp_runtime::traits::Block;

use crate::network::{Event, EventStream, Network, NetworkSender, Protocol, RequestBlocks};

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

    fn is_major_syncing(&self) -> bool {
        NetworkService::is_major_syncing(self)
    }
}

/// Name of the network protocol used by Aleph Zero. This is how messages
/// are subscribed to ensure that we are gossiping and communicating with our
/// own network.
const AUTHENTICATION_PROTOCOL_NAME: &str = "/aleph/1";

/// Returns the canonical name of the protocol.
pub fn protocol_name(protocol: &Protocol) -> Cow<'static, str> {
    use Protocol::*;
    match protocol {
        Authentication => Cow::Borrowed(AUTHENTICATION_PROTOCOL_NAME),
    }
}

/// Attempts to convert the protocol name to a protocol.
fn to_protocol(protocol_name: &str) -> Result<Protocol, ()> {
    match protocol_name {
        AUTHENTICATION_PROTOCOL_NAME => Ok(Protocol::Authentication),
        _ => Err(()),
    }
}

#[derive(Debug)]
pub enum SenderError {
    CannotCreateSender(PeerId, Protocol),
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
    notification_sender: Box<dyn NotificationSender>,
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
            .send(data.into())
            .map_err(|_| SenderError::LostConnectionToPeerReady(self.peer_id))
    }
}

type NetworkEventStream = Pin<Box<dyn Stream<Item = SubstrateEvent> + Send>>;

#[async_trait]
impl EventStream<Multiaddr, PeerId> for NetworkEventStream {
    async fn next_event(&mut self) -> Option<Event<Multiaddr, PeerId>> {
        use Event::*;
        use SubstrateEvent::*;
        loop {
            match self.next().await {
                Some(event) => match event {
                    SyncConnected { remote } => {
                        return Some(Connected(
                            iter::once(MultiaddressProtocol::P2p(remote.into())).collect(),
                        ))
                    }
                    SyncDisconnected { remote } => return Some(Disconnected(remote)),
                    NotificationStreamOpened {
                        remote, protocol, ..
                    } => match to_protocol(protocol.as_ref()) {
                        Ok(protocol) => return Some(StreamOpened(remote, protocol)),
                        Err(_) => continue,
                    },
                    NotificationStreamClosed { remote, protocol } => {
                        match to_protocol(protocol.as_ref()) {
                            Ok(protocol) => return Some(StreamClosed(remote, protocol)),
                            Err(_) => continue,
                        }
                    }
                    NotificationsReceived { messages, .. } => {
                        return Some(Messages(
                            messages
                                .into_iter()
                                .filter_map(|(protocol, data)| {
                                    match to_protocol(protocol.as_ref()) {
                                        Ok(protocol) => Some((protocol, data)),
                                        // This might end with us returning an empty vec, but it's probably not
                                        // worth it to handle this situation here.
                                        Err(_) => None,
                                    }
                                })
                                .collect(),
                        ));
                    }
                    Dht(_) => continue,
                },
                None => return None,
            }
        }
    }
}

impl<B: Block, H: ExHashT> Network for Arc<NetworkService<B, H>> {
    type SenderError = SenderError;
    type NetworkSender = SubstrateNetworkSender;
    type PeerId = PeerId;
    type Multiaddress = Multiaddr;
    type EventStream = NetworkEventStream;

    fn event_stream(&self) -> Self::EventStream {
        Box::pin(self.as_ref().event_stream("aleph-network"))
    }

    fn sender(
        &self,
        peer_id: Self::PeerId,
        protocol: Protocol,
    ) -> Result<Self::NetworkSender, Self::SenderError> {
        Ok(SubstrateNetworkSender {
            // Currently method `notification_sender` does not distinguish whether we are not connected to the peer
            // or there is no such protocol so we need to have this worthless `SenderError::CannotCreateSender` error here
            notification_sender: self
                .notification_sender(peer_id, protocol_name(&protocol))
                .map_err(|_| SenderError::CannotCreateSender(peer_id, protocol))?,
            peer_id,
        })
    }

    fn add_reserved(&self, addresses: HashSet<Self::Multiaddress>, protocol: Protocol) {
        if let Err(e) = self
            .add_peers_to_reserved_set(protocol_name(&protocol), addresses.into_iter().collect())
        {
            error!(target: "aleph-network", "add_reserved failed: {}", e);
        }
    }

    fn remove_reserved(&self, peers: HashSet<Self::PeerId>, protocol: Protocol) {
        let addresses = peers.into_iter().collect();
        self.remove_peers_from_reserved_set(protocol_name(&protocol), addresses);
    }
}
