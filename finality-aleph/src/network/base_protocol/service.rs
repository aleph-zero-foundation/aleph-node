use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    iter,
    sync::{
        atomic::{AtomicBool, AtomicUsize},
        Arc,
    },
};

use futures::stream::StreamExt;
use log::{debug, error, trace, warn};
use sc_network::{
    config::{NetworkConfiguration, NotificationService},
    multiaddr::Protocol as MultiaddressProtocol,
    service::traits::{NotificationEvent as SubstrateEvent, ValidationResult},
    Multiaddr, NetworkPeers, NetworkService, ProtocolName,
};
use sc_network_sync::{
    service::syncing_service::ToServiceCommand, types::SyncEvent, SyncingService,
};
use sc_utils::mpsc::{tracing_unbounded, TracingUnboundedReceiver, TracingUnboundedSender};
use sp_runtime::traits::{Block, Header};

use crate::{
    network::base_protocol::{handler::Handler, LOG_TARGET},
    BlockHash, BlockNumber,
};

#[derive(Debug)]
pub enum Error {
    NoIncomingCommands,
    NoNetworkEvents,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        use Error::*;
        match self {
            NoIncomingCommands => write!(f, "channel with commands from user closed"),
            NoNetworkEvents => write!(f, "channel with events from network closed"),
        }
    }
}

/// A service that needs to be run to have the base protocol of the network work.
/// It also responds to some external requests, but mostly by mocking them.
pub struct Service<B>
where
    B: Block<Hash = BlockHash>,
    B::Header: Header<Number = BlockNumber>,
{
    handler: Handler<B>,
    protocol_names: Vec<ProtocolName>,
    network: Arc<NetworkService<B, B::Hash>>,
    commands_from_user: TracingUnboundedReceiver<ToServiceCommand<B>>,
    events_for_users: Vec<TracingUnboundedSender<SyncEvent>>,
    events_from_network: Box<dyn NotificationService>,
}

impl<B> Service<B>
where
    B: Block<Hash = BlockHash>,
    B::Header: Header<Number = BlockNumber>,
{
    /// Create a new service.
    // TODO(A0-3886): This shouldn't need to return the substrate type after replacing RPCs.
    // In particular, it shouldn't depend on `B`. This is also the only reason why
    // the `major_sync` argument is needed.
    pub fn new(
        major_sync: Arc<AtomicBool>,
        genesis_hash: B::Hash,
        net_config: &NetworkConfiguration,
        protocol_names: Vec<ProtocolName>,
        network: Arc<NetworkService<B, BlockHash>>,
        events_from_network: Box<dyn NotificationService>,
    ) -> (Self, Arc<SyncingService<B>>) {
        let (commands_for_service, commands_from_user) =
            tracing_unbounded("mpsc_base_protocol", 100_000);
        (
            Service {
                handler: Handler::new(genesis_hash, net_config),
                protocol_names,
                network,
                commands_from_user,
                events_for_users: Vec::new(),
                events_from_network,
            },
            Arc::new(SyncingService::new(
                commands_for_service,
                // We don't care about this one, so a dummy value.
                Arc::new(AtomicUsize::new(0)),
                major_sync,
            )),
        )
    }

    fn handle_command(&mut self, command: ToServiceCommand<B>) {
        use ToServiceCommand::*;
        match command {
            EventStream(events_for_user) => self.events_for_users.push(events_for_user),
            PeersInfo(response) => {
                if response.send(self.handler.peers_info()).is_err() {
                    debug!(
                        target: LOG_TARGET,
                        "Failed to send response to peers info request."
                    );
                }
            }
            BestSeenBlock(response) => {
                if response.send(None).is_err() {
                    debug!(
                        target: LOG_TARGET,
                        "Failed to send response to best block request."
                    );
                }
            }
            Status(_) => {
                // We are explicitly dropping the response channel to cause an `Err(())` to be returned in the interface, as this produces the desired results for us.
                trace!(target: LOG_TARGET, "Got status request, ignoring.");
            }
            _ => {
                warn!(target: LOG_TARGET, "Got unexpected service command.");
            }
        }
    }

    fn handle_network_event(&mut self, event: SubstrateEvent) {
        use SubstrateEvent::*;
        match event {
            ValidateInboundSubstream {
                peer,
                handshake,
                result_tx,
            } => {
                let result = match self.handler.verify_inbound_connection(peer, handshake) {
                    Ok(()) => ValidationResult::Accept,
                    Err(e) => {
                        debug!(target: LOG_TARGET, "Rejecting incoming substream: {}.", e);
                        ValidationResult::Reject
                    }
                };
                if result_tx.send(result).is_err() {
                    debug!(
                        target: LOG_TARGET,
                        "Failed to send response to inbound substream validation request."
                    );
                }
            }
            NotificationStreamOpened {
                peer,
                handshake,
                direction,
                negotiated_fallback: _,
            } => match self.handler.on_peer_connect(peer, handshake, direction) {
                Ok(()) => {
                    let multiaddress: Multiaddr =
                        iter::once(MultiaddressProtocol::P2p(peer.into())).collect();
                    trace!(target: LOG_TARGET, "Connect event from address {:?}.", multiaddress);
                    for name in &self.protocol_names {
                        if let Err(e) = self.network.add_peers_to_reserved_set(
                            name.clone(),
                            iter::once(multiaddress.clone()).collect(),
                        ) {
                            error!(target: LOG_TARGET, "Adding peer to the {} reserved set failed: {}.", name, e);
                        }
                    }
                    self.events_for_users.retain(|for_user| {
                        for_user
                            .unbounded_send(SyncEvent::PeerConnected(peer))
                            .is_ok()
                    });
                }
                Err(e) => debug!(target:LOG_TARGET, "Failed to accept connection: {}.", e),
            },
            NotificationStreamClosed { peer } => {
                trace!(target: LOG_TARGET, "Disconnect event for peer {:?}", peer);
                if let Err(e) = self.handler.on_peer_disconnect(peer) {
                    warn!(target: LOG_TARGET, "Problem removing disconnecting peer: {e}.");
                }
                let addresses: Vec<_> = iter::once(peer).collect();
                for name in &self.protocol_names {
                    if let Err(e) = self
                        .network
                        .remove_peers_from_reserved_set(name.clone(), addresses.clone())
                    {
                        warn!(target: LOG_TARGET, "Removing peer from the {} reserved set failed: {}", name, e)
                    }
                }
                self.events_for_users.retain(|for_user| {
                    for_user
                        .unbounded_send(SyncEvent::PeerDisconnected(peer))
                        .is_ok()
                });
            }
            NotificationReceived { peer, .. } => {
                debug!(target: LOG_TARGET, "Received unexpected message in the base protocol from {}.", peer)
            }
        }
    }

    /// Run the service managing the base protocol.
    pub async fn run(mut self) -> Result<(), Error> {
        use Error::*;
        loop {
            tokio::select! {
                command = self.commands_from_user.next() => self.handle_command(command.ok_or(NoIncomingCommands)?),
                event = self.events_from_network.next_event() => self.handle_network_event(event.ok_or(NoNetworkEvents)?),
            }
        }
    }
}
