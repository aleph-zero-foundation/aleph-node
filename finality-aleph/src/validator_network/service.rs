use std::{collections::HashSet, fmt::Debug};

use aleph_primitives::AuthorityId;
use futures::{
    channel::{mpsc, oneshot},
    StreamExt,
};
use log::{debug, info, trace, warn};
use tokio::time;

use crate::{
    crypto::AuthorityPen,
    validator_network::{
        incoming::incoming,
        manager::{AddResult, LegacyManager, Manager},
        outgoing::outgoing,
        protocols::{ConnectionType, ResultForService},
        Data, Dialer, Listener, Network,
    },
    SpawnTaskHandle, STATUS_REPORT_INTERVAL,
};

enum ServiceCommand<D: Data, A: Data> {
    AddConnection(AuthorityId, Vec<A>),
    DelConnection(AuthorityId),
    SendData(D, AuthorityId),
}

struct ServiceInterface<D: Data, A: Data> {
    commands_for_service: mpsc::UnboundedSender<ServiceCommand<D, A>>,
    next_from_service: mpsc::UnboundedReceiver<D>,
}

#[async_trait::async_trait]
impl<D: Data, A: Data> Network<A, D> for ServiceInterface<D, A> {
    /// Add the peer to the set of connected peers.
    fn add_connection(&mut self, peer: AuthorityId, addresses: Vec<A>) {
        if self
            .commands_for_service
            .unbounded_send(ServiceCommand::AddConnection(peer, addresses))
            .is_err()
        {
            info!(target: "validator-network", "Service is dead.");
        };
    }

    /// Remove the peer from the set of connected peers and close the connection.
    fn remove_connection(&mut self, peer: AuthorityId) {
        if self
            .commands_for_service
            .unbounded_send(ServiceCommand::DelConnection(peer))
            .is_err()
        {
            info!(target: "validator-network", "Service is dead.");
        };
    }

    /// Send a message to a single peer.
    /// This function should be implemented in a non-blocking manner.
    fn send(&self, data: D, recipient: AuthorityId) {
        if self
            .commands_for_service
            .unbounded_send(ServiceCommand::SendData(data, recipient))
            .is_err()
        {
            info!(target: "validator-network", "Service is dead.");
        };
    }

    /// Receive a message from the network.
    async fn next(&mut self) -> Option<D> {
        self.next_from_service.next().await
    }
}

/// A service that has to be run for the validator network to work.
pub struct Service<D: Data, A: Data, ND: Dialer<A>, NL: Listener> {
    commands_from_interface: mpsc::UnboundedReceiver<ServiceCommand<D, A>>,
    next_to_interface: mpsc::UnboundedSender<D>,
    manager: Manager<A, D>,
    dialer: ND,
    listener: NL,
    spawn_handle: SpawnTaskHandle,
    authority_pen: AuthorityPen,
    // Backwards compatibility with the one-sided connections, remove when no longer needed.
    legacy_connected: HashSet<AuthorityId>,
    legacy_manager: LegacyManager<A, D>,
}

impl<D: Data, A: Data + Debug, ND: Dialer<A>, NL: Listener> Service<D, A, ND, NL> {
    /// Create a new validator network service plus an interface for interacting with it.
    pub fn new(
        dialer: ND,
        listener: NL,
        authority_pen: AuthorityPen,
        spawn_handle: SpawnTaskHandle,
    ) -> (Self, impl Network<A, D>) {
        // Channel for sending commands between the service and interface
        let (commands_for_service, commands_from_interface) = mpsc::unbounded();
        // Channel for receiving data from the network
        let (next_to_interface, next_from_service) = mpsc::unbounded();
        (
            Self {
                commands_from_interface,
                next_to_interface,
                manager: Manager::new(authority_pen.authority_id()),
                dialer,
                listener,
                spawn_handle,
                authority_pen,
                legacy_connected: HashSet::new(),
                legacy_manager: LegacyManager::new(),
            },
            ServiceInterface {
                commands_for_service,
                next_from_service,
            },
        )
    }

    fn spawn_new_outgoing(
        &self,
        peer_id: AuthorityId,
        addresses: Vec<A>,
        result_for_parent: mpsc::UnboundedSender<ResultForService<D>>,
    ) {
        let authority_pen = self.authority_pen.clone();
        let dialer = self.dialer.clone();
        let next_to_interface = self.next_to_interface.clone();
        self.spawn_handle
            .spawn("aleph/validator_network_outgoing", None, async move {
                outgoing(
                    authority_pen,
                    peer_id,
                    dialer,
                    addresses,
                    result_for_parent,
                    next_to_interface,
                )
                .await;
            });
    }

    fn spawn_new_incoming(
        &self,
        stream: NL::Connection,
        result_for_parent: mpsc::UnboundedSender<ResultForService<D>>,
    ) {
        let authority_pen = self.authority_pen.clone();
        let next_to_interface = self.next_to_interface.clone();
        self.spawn_handle
            .spawn("aleph/validator_network_incoming", None, async move {
                incoming(authority_pen, stream, result_for_parent, next_to_interface).await;
            });
    }

    fn peer_addresses(&self, peer_id: &AuthorityId) -> Option<Vec<A>> {
        match self.legacy_connected.contains(peer_id) {
            true => self.legacy_manager.peer_addresses(peer_id),
            false => self.manager.peer_addresses(peer_id),
        }
    }

    fn add_connection(
        &mut self,
        peer_id: AuthorityId,
        data_for_network: mpsc::UnboundedSender<D>,
        connection_type: ConnectionType,
    ) -> AddResult {
        use ConnectionType::*;
        match connection_type {
            New => {
                // If we are adding a non-legacy connection we want to ensure it's not marked as
                // such. This should only matter if a peer initially used the legacy protocol but
                // now upgraded, otherwise this is unnecessary busywork, but what can you do.
                self.unmark_legacy(&peer_id);
                self.manager.add_connection(peer_id, data_for_network)
            }
            LegacyIncoming => self.legacy_manager.add_incoming(peer_id, data_for_network),
            LegacyOutgoing => self.legacy_manager.add_outgoing(peer_id, data_for_network),
        }
    }

    // Mark a peer as legacy and return whether it is the first time we do so.
    fn mark_legacy(&mut self, peer_id: &AuthorityId) -> bool {
        self.manager.remove_peer(peer_id);
        self.legacy_connected.insert(peer_id.clone())
    }

    // Unmark a peer as legacy, putting it back in the normal set.
    fn unmark_legacy(&mut self, peer_id: &AuthorityId) {
        self.legacy_connected.remove(peer_id);
        // Put it back if we still want to be connected.
        if let Some(addresses) = self.legacy_manager.peer_addresses(peer_id) {
            self.manager.add_peer(peer_id.clone(), addresses);
        }
    }

    // Checks whether this peer should now be marked as one using the legacy protocol and handled
    // accordingly. Returns whether we should spawn a new connection worker because of that.
    fn check_for_legacy(&mut self, peer_id: &AuthorityId, connection_type: ConnectionType) -> bool {
        use ConnectionType::*;
        match connection_type {
            LegacyIncoming => self.mark_legacy(peer_id),
            LegacyOutgoing => {
                self.mark_legacy(peer_id);
                false
            }
            // We don't unmark here, because we always return New when a connection
            // fails early, and in such cases we want to keep the previous guess as to
            // how we want to connect. We unmark once we successfully negotiate and add
            // a connection.
            New => false,
        }
    }

    /// Run the service until a signal from exit.
    pub async fn run(mut self, mut exit: oneshot::Receiver<()>) {
        let mut status_ticker = time::interval(STATUS_REPORT_INTERVAL);
        let (result_for_parent, mut worker_results) = mpsc::unbounded();
        use ServiceCommand::*;
        loop {
            tokio::select! {
                // got new incoming connection from the listener - spawn an incoming worker
                maybe_stream = self.listener.accept() => match maybe_stream {
                    Ok(stream) => self.spawn_new_incoming(stream, result_for_parent.clone()),
                    Err(e) => warn!(target: "validator-network", "Listener failed to accept connection: {}", e),
                },
                // got a new command from the interface
                Some(command) = self.commands_from_interface.next() => match command {
                    // register new peer in manager or update its list of addresses if already there
                    // spawn a worker managing outgoing connection if the peer was not known
                    AddConnection(peer_id, addresses) => {
                        // we add all the peers to the legacy manager so we don't lose the
                        // addresses, but only care about its opinion when it turns out we have to
                        // in particular the first time we add a peer we never know whether it
                        // requires legacy connecting, so we only attempt to connect to it if the
                        // new criterion is satisfied, otherwise we wait for it to connect to us
                        self.legacy_manager.add_peer(peer_id.clone(), addresses.clone());
                        if self.manager.add_peer(peer_id.clone(), addresses.clone()) {
                            self.spawn_new_outgoing(peer_id, addresses, result_for_parent.clone());
                        };
                    },
                    // remove the peer from the manager all workers will be killed automatically, due to closed channels
                    DelConnection(peer_id) => {
                        self.manager.remove_peer(&peer_id);
                        self.legacy_manager.remove_peer(&peer_id);
                        self.legacy_connected.remove(&peer_id);
                    },
                    // pass the data to the manager
                    SendData(data, peer_id) => {
                        match self.legacy_connected.contains(&peer_id) {
                            true => match self.legacy_manager.send_to(&peer_id, data) {
                                Ok(_) => trace!(target: "validator-network", "Sending data to {} through legacy.", peer_id),
                                Err(e) => trace!(target: "validator-network", "Failed sending to {} through legacy: {}", peer_id, e),
                            },
                            false => match self.manager.send_to(&peer_id, data) {
                                Ok(_) => trace!(target: "validator-network", "Sending data to {}.", peer_id),
                                Err(e) => trace!(target: "validator-network", "Failed sending to {}: {}", peer_id, e),
                            },
                        }
                    },
                },
                // received information from a spawned worker managing a connection
                // check if we still want to be connected to the peer, and if so, spawn a new worker or actually add proper connection
                Some((peer_id, maybe_data_for_network, connection_type)) = worker_results.next() => {
                    if self.check_for_legacy(&peer_id, connection_type) {
                        match self.legacy_manager.peer_addresses(&peer_id) {
                            Some(addresses) => self.spawn_new_outgoing(peer_id.clone(), addresses, result_for_parent.clone()),
                            None => {
                                // We received a result from a worker we are no longer interested
                                // in.
                                self.legacy_connected.remove(&peer_id);
                            },
                        }
                    }
                    use AddResult::*;
                    match maybe_data_for_network {
                        Some(data_for_network) => match self.add_connection(peer_id.clone(), data_for_network, connection_type) {
                            Uninterested => warn!(target: "validator-network", "Established connection with peer {} for unknown reasons.", peer_id),
                            Added => info!(target: "validator-network", "New connection with peer {}.", peer_id),
                            Replaced => info!(target: "validator-network", "Replaced connection with peer {}.", peer_id),
                        },
                        None => if let Some(addresses) = self.peer_addresses(&peer_id) {
                            self.spawn_new_outgoing(peer_id, addresses, result_for_parent.clone());
                        }
                    }
                },
                // periodically reporting what we are trying to do
                _ = status_ticker.tick() => {
                    info!(target: "validator-network", "Validator Network status: {}", self.manager.status_report());
                    debug!(target: "validator-network", "Validator Network legacy status: {}", self.legacy_manager.status_report());
                }
                // received exit signal, stop the network
                // all workers will be killed automatically after the manager gets dropped
                _ = &mut exit => break,
            };
        }
    }
}
