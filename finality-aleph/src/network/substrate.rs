use std::{collections::HashMap, iter, pin::Pin, sync::Arc};

use async_trait::async_trait;
use futures::stream::{Fuse, Stream, StreamExt};
use log::{error, trace, warn};
pub use sc_network::PeerId;
use sc_network::{
    multiaddr::Protocol as MultiaddressProtocol,
    service::traits::{NotificationEvent as SubstrateEvent, ValidationResult},
    Multiaddr, NetworkPeers, NetworkService, ProtocolName,
};
use sc_network_common::ExHashT;
use sc_network_sync::{SyncEvent, SyncEventStream, SyncingService};
use sp_runtime::traits::Block;

use crate::network::gossip::{Event, EventStream, Protocol};

/// Name of the network protocol used by Aleph Zero to disseminate validator
/// authentications.
const AUTHENTICATION_PROTOCOL_NAME: &str = "/auth/0";

/// Name of the network protocol used by Aleph Zero to synchronize the block state.
const BLOCK_SYNC_PROTOCOL_NAME: &str = "/sync/0";

/// Convert protocols to their names and vice versa.
#[derive(Clone)]
pub struct ProtocolNaming {
    authentication_name: ProtocolName,
    block_sync_name: ProtocolName,
}

impl ProtocolNaming {
    /// Create a new protocol naming scheme with the given chain prefix.
    pub fn new(chain_prefix: String) -> Self {
        let authentication_name: ProtocolName =
            format!("{chain_prefix}{AUTHENTICATION_PROTOCOL_NAME}").into();
        let mut protocols_by_name = HashMap::new();
        protocols_by_name.insert(authentication_name.clone(), Protocol::Authentication);
        let block_sync_name: ProtocolName =
            format!("{chain_prefix}{BLOCK_SYNC_PROTOCOL_NAME}").into();
        protocols_by_name.insert(block_sync_name.clone(), Protocol::BlockSync);
        ProtocolNaming {
            authentication_name,
            block_sync_name,
        }
    }

    /// Returns the canonical name of the protocol.
    pub fn protocol_name(&self, protocol: &Protocol) -> ProtocolName {
        use Protocol::*;
        match protocol {
            Authentication => self.authentication_name.clone(),
            BlockSync => self.block_sync_name.clone(),
        }
    }

    /// Returns the fallback names of the protocol.
    pub fn fallback_protocol_names(&self, _protocol: &Protocol) -> Vec<ProtocolName> {
        Vec::new()
    }
}

/// A struct holding NotificationService per protocol we use.
pub struct NotificationServices {
    pub authentication: Box<dyn sc_network::config::NotificationService>,
    pub sync: Box<dyn sc_network::config::NotificationService>,
}

pub struct NetworkEventStream<B: Block, H: ExHashT> {
    sync_stream: Fuse<Pin<Box<dyn Stream<Item = SyncEvent> + Send>>>,
    notifications: NotificationServices,
    naming: ProtocolNaming,
    network: Arc<NetworkService<B, H>>,
}

impl<B: Block, H: ExHashT> NetworkEventStream<B, H> {
    pub fn new(
        network: Arc<NetworkService<B, H>>,
        sync_network: Arc<SyncingService<B>>,
        naming: ProtocolNaming,
        notifications: NotificationServices,
    ) -> Self {
        Self {
            sync_stream: sync_network.event_stream("aleph-syncing-network").fuse(),
            notifications,
            naming,
            network,
        }
    }
}

#[async_trait]
impl<B: Block, H: ExHashT> EventStream<PeerId> for NetworkEventStream<B, H> {
    async fn next_event(&mut self) -> Option<Event<PeerId>> {
        use Event::*;
        use SyncEvent::*;
        loop {
            tokio::select! {
                Some(event) = self.notifications.sync.next_event() => {
                    use SubstrateEvent::*;
                    match event {
                        ValidateInboundSubstream {
                            peer: _,
                            handshake: _,
                            result_tx,
                        } => {
                            let _ = result_tx.send(ValidationResult::Accept);
                            continue
                        },
                        NotificationStreamOpened {
                            peer,
                            ..
                        } => {
                            match self.notifications.sync.message_sink(&peer) {
                                Some(sink) => return Some(StreamOpened(peer, Protocol::BlockSync, sink)),
                                None => {
                                    warn!(target: "aleph-network", "Received NotificationStreamOpened from peer {peer:?} in BlockSync protocol, but could not create MessageSink.");
                                    continue;
                                }
                            }
                        },
                        NotificationStreamClosed {
                            peer,
                        } => {
                            return Some(StreamClosed(peer, Protocol::BlockSync));
                        },
                        NotificationReceived {
                            peer,
                            notification,
                        } => {
                            return Some(Messages(
                                peer,
                                vec![(Protocol::BlockSync, notification.into())],
                            ));
                        },
                    }
                },

                Some(event) = self.notifications.authentication.next_event() => {
                    use SubstrateEvent::*;
                    match event {
                        ValidateInboundSubstream {
                            peer: _,
                            handshake: _,
                            result_tx,
                        } => {
                            let _ = result_tx.send(ValidationResult::Accept);
                            continue
                        },
                        NotificationStreamOpened {
                            peer,
                            ..
                        } => {
                            match self.notifications.authentication.message_sink(&peer) {
                                Some(sink) => return Some(StreamOpened(peer, Protocol::Authentication, sink)),
                                None => {
                                    warn!(target: "aleph-network", "Received NotificationStreamOpened from peer {peer:?} in Authentication protocol, but could not create MessageSink.");
                                    continue;
                                }
                            }
                        },
                        NotificationStreamClosed {
                            peer,
                        } => {
                            return Some(StreamClosed(peer, Protocol::Authentication));
                        },
                        NotificationReceived {
                            peer,
                            notification,
                        } => {
                            return Some(Messages(
                                peer,
                                vec![(Protocol::Authentication, notification.into())],
                            ));
                        },
                    }
                },

                Some(event) = self.sync_stream.next() => {
                    match event {
                        PeerConnected(remote) => {
                            let multiaddress: Multiaddr =
                                iter::once(MultiaddressProtocol::P2p(remote.into())).collect();
                            trace!(target: "aleph-network", "Connected event from address {:?}", multiaddress);
                            if let Err(e) = self.network.add_peers_to_reserved_set(
                                self.naming.protocol_name(&Protocol::Authentication),
                                iter::once(multiaddress.clone()).collect(),
                            ) {
                                error!(target: "aleph-network", "add_reserved failed for authentications: {}", e);
                            }
                            if let Err(e) = self.network.add_peers_to_reserved_set(
                                self.naming.protocol_name(&Protocol::BlockSync),
                                iter::once(multiaddress).collect(),
                            ) {
                                error!(target: "aleph-network", "add_reserved failed for block sync: {}", e);
                            }
                            continue;
                        }
                        PeerDisconnected(remote) => {
                            trace!(target: "aleph-network", "Disconnected event for peer {:?}", remote);
                            let addresses: Vec<_> = iter::once(remote).collect();
                            if let Err(e) = self.network.remove_peers_from_reserved_set(
                                self.naming.protocol_name(&Protocol::Authentication),
                                addresses.clone(),
                            ) {
                                warn!(target: "aleph-network", "Error while removing peer from Protocol::Authentication reserved set: {}", e)
                            }
                            if let Err(e) = self.network.remove_peers_from_reserved_set(
                                self.naming.protocol_name(&Protocol::BlockSync),
                                addresses,
                            ) {
                                warn!(target: "aleph-network", "Error while removing peer from Protocol::BlockSync reserved set: {}", e)
                            }
                            continue;
                        }
                    }
                },

                else => return None,
            }
        }
    }
}
