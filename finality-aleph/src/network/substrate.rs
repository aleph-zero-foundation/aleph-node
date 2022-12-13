use std::{fmt, iter, pin::Pin, sync::Arc};

use async_trait::async_trait;
use futures::stream::{Stream, StreamExt};
use log::{error, trace};
use sc_consensus::JustificationSyncLink;
use sc_network::{
    multiaddr::Protocol as MultiaddressProtocol, Event as SubstrateEvent, NetworkService,
    NetworkSyncForkRequest, PeerId,
};
use sc_network_common::{
    protocol::ProtocolName,
    service::{NetworkEventStream as _, NetworkNotification, NetworkPeers, NotificationSender},
    ExHashT,
};
use sp_api::NumberFor;
use sp_consensus::SyncOracle;
use sp_runtime::traits::Block;

use crate::network::{
    gossip::{Event, EventStream, NetworkSender, Protocol, RawNetwork},
    RequestBlocks,
};

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
pub fn protocol_name(protocol: &Protocol) -> ProtocolName {
    use Protocol::*;
    match protocol {
        Authentication => AUTHENTICATION_PROTOCOL_NAME.into(),
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

pub struct NetworkEventStream<B: Block, H: ExHashT> {
    stream: Pin<Box<dyn Stream<Item = SubstrateEvent> + Send>>,
    network: Arc<NetworkService<B, H>>,
}

#[async_trait]
impl<B: Block, H: ExHashT> EventStream<PeerId> for NetworkEventStream<B, H> {
    async fn next_event(&mut self) -> Option<Event<PeerId>> {
        use Event::*;
        use SubstrateEvent::*;
        loop {
            match self.stream.next().await {
                Some(event) => match event {
                    SyncConnected { remote } => {
                        let multiaddress =
                            iter::once(MultiaddressProtocol::P2p(remote.into())).collect();
                        trace!(target: "aleph-network", "Connected event from address {:?}", multiaddress);
                        if let Err(e) = self.network.add_peers_to_reserved_set(
                            protocol_name(&Protocol::Authentication),
                            iter::once(multiaddress).collect(),
                        ) {
                            error!(target: "aleph-network", "add_reserved failed: {}", e);
                        }
                        continue;
                    }
                    SyncDisconnected { remote } => {
                        trace!(target: "aleph-network", "Disconnected event for peer {:?}", remote);
                        let addresses = iter::once(remote).collect();
                        self.network.remove_peers_from_reserved_set(
                            protocol_name(&Protocol::Authentication),
                            addresses,
                        );
                        continue;
                    }
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

impl<B: Block, H: ExHashT> RawNetwork for Arc<NetworkService<B, H>> {
    type SenderError = SenderError;
    type NetworkSender = SubstrateNetworkSender;
    type PeerId = PeerId;
    type EventStream = NetworkEventStream<B, H>;

    fn event_stream(&self) -> Self::EventStream {
        NetworkEventStream {
            stream: Box::pin(self.as_ref().event_stream("aleph-network")),
            network: self.clone(),
        }
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
}
