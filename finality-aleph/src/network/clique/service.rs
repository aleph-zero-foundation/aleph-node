use std::{collections::HashSet, fmt::Debug};

use futures::{
    channel::{mpsc, oneshot},
    StreamExt,
};
use log::{debug, info, trace, warn};
use tokio::time;

use crate::{
    network::{
        clique::{
            incoming::incoming,
            manager::{AddResult, LegacyManager, Manager},
            outgoing::outgoing,
            protocols::{ConnectionType, ResultForService},
            Dialer, Listener, Network, PublicKey, SecretKey, LOG_TARGET,
        },
        Data, PeerId,
    },
    SpawnTaskHandle, STATUS_REPORT_INTERVAL,
};

enum ServiceCommand<PK: PublicKey, D: Data, A: Data> {
    AddConnection(PK, A),
    DelConnection(PK),
    SendData(D, PK),
}

struct ServiceInterface<PK: PublicKey, D: Data, A: Data> {
    commands_for_service: mpsc::UnboundedSender<ServiceCommand<PK, D, A>>,
    next_from_service: mpsc::UnboundedReceiver<D>,
}

#[async_trait::async_trait]
impl<PK: PublicKey, D: Data, A: Data> Network<PK, A, D> for ServiceInterface<PK, D, A> {
    /// Add the peer to the set of connected peers.
    fn add_connection(&mut self, peer: PK, address: A) {
        if self
            .commands_for_service
            .unbounded_send(ServiceCommand::AddConnection(peer, address))
            .is_err()
        {
            info!(target: LOG_TARGET, "Service is dead.");
        };
    }

    /// Remove the peer from the set of connected peers and close the connection.
    fn remove_connection(&mut self, peer: PK) {
        if self
            .commands_for_service
            .unbounded_send(ServiceCommand::DelConnection(peer))
            .is_err()
        {
            info!(target: LOG_TARGET, "Service is dead.");
        };
    }

    /// Send a message to a single peer.
    /// This function should be implemented in a non-blocking manner.
    fn send(&self, data: D, recipient: PK) {
        if self
            .commands_for_service
            .unbounded_send(ServiceCommand::SendData(data, recipient))
            .is_err()
        {
            info!(target: LOG_TARGET, "Service is dead.");
        };
    }

    /// Receive a message from the network.
    async fn next(&mut self) -> Option<D> {
        self.next_from_service.next().await
    }
}

/// A service that has to be run for the clique network to work.
pub struct Service<SK: SecretKey, D: Data, A: Data, ND: Dialer<A>, NL: Listener>
where
    SK::PublicKey: PeerId,
{
    commands_from_interface: mpsc::UnboundedReceiver<ServiceCommand<SK::PublicKey, D, A>>,
    next_to_interface: mpsc::UnboundedSender<D>,
    manager: Manager<SK::PublicKey, A, D>,
    dialer: ND,
    listener: NL,
    spawn_handle: SpawnTaskHandle,
    secret_key: SK,
    // Backwards compatibility with the one-sided connections, remove when no longer needed.
    legacy_connected: HashSet<SK::PublicKey>,
    legacy_manager: LegacyManager<SK::PublicKey, A, D>,
}

impl<SK: SecretKey, D: Data, A: Data + Debug, ND: Dialer<A>, NL: Listener> Service<SK, D, A, ND, NL>
where
    SK::PublicKey: PeerId,
{
    /// Create a new clique network service plus an interface for interacting with it.
    pub fn new(
        dialer: ND,
        listener: NL,
        secret_key: SK,
        spawn_handle: SpawnTaskHandle,
    ) -> (Self, impl Network<SK::PublicKey, A, D>) {
        // Channel for sending commands between the service and interface
        let (commands_for_service, commands_from_interface) = mpsc::unbounded();
        // Channel for receiving data from the network
        let (next_to_interface, next_from_service) = mpsc::unbounded();
        (
            Self {
                commands_from_interface,
                next_to_interface,
                manager: Manager::new(secret_key.public_key()),
                dialer,
                listener,
                spawn_handle,
                secret_key,
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
        public_key: SK::PublicKey,
        address: A,
        result_for_parent: mpsc::UnboundedSender<ResultForService<SK::PublicKey, D>>,
    ) {
        let secret_key = self.secret_key.clone();
        let dialer = self.dialer.clone();
        let next_to_interface = self.next_to_interface.clone();
        self.spawn_handle
            .spawn("aleph/clique_network_outgoing", None, async move {
                outgoing(
                    secret_key,
                    public_key,
                    dialer,
                    address,
                    result_for_parent,
                    next_to_interface,
                )
                .await;
            });
    }

    fn spawn_new_incoming(
        &self,
        stream: NL::Connection,
        result_for_parent: mpsc::UnboundedSender<ResultForService<SK::PublicKey, D>>,
    ) {
        let secret_key = self.secret_key.clone();
        let next_to_interface = self.next_to_interface.clone();
        self.spawn_handle
            .spawn("aleph/clique_network_incoming", None, async move {
                incoming(secret_key, stream, result_for_parent, next_to_interface).await;
            });
    }

    fn peer_address(&self, public_key: &SK::PublicKey) -> Option<A> {
        match self.legacy_connected.contains(public_key) {
            true => self.legacy_manager.peer_address(public_key),
            false => self.manager.peer_address(public_key),
        }
    }

    fn add_connection(
        &mut self,
        public_key: SK::PublicKey,
        data_for_network: mpsc::UnboundedSender<D>,
        connection_type: ConnectionType,
    ) -> AddResult {
        use ConnectionType::*;
        match connection_type {
            New => {
                // If we are adding a non-legacy connection we want to ensure it's not marked as
                // such. This should only matter if a peer initially used the legacy protocol but
                // now upgraded, otherwise this is unnecessary busywork, but what can you do.
                self.unmark_legacy(&public_key);
                self.manager.add_connection(public_key, data_for_network)
            }
            LegacyIncoming => self
                .legacy_manager
                .add_incoming(public_key, data_for_network),
            LegacyOutgoing => self
                .legacy_manager
                .add_outgoing(public_key, data_for_network),
        }
    }

    // Mark a peer as legacy and return whether it is the first time we do so.
    fn mark_legacy(&mut self, public_key: &SK::PublicKey) -> bool {
        self.manager.remove_peer(public_key);
        self.legacy_connected.insert(public_key.clone())
    }

    // Unmark a peer as legacy, putting it back in the normal set.
    fn unmark_legacy(&mut self, public_key: &SK::PublicKey) {
        self.legacy_connected.remove(public_key);
        // Put it back if we still want to be connected.
        if let Some(address) = self.legacy_manager.peer_address(public_key) {
            self.manager.add_peer(public_key.clone(), address);
        }
    }

    // Checks whether this peer should now be marked as one using the legacy protocol and handled
    // accordingly. Returns whether we should spawn a new connection worker because of that.
    fn check_for_legacy(
        &mut self,
        public_key: &SK::PublicKey,
        connection_type: ConnectionType,
    ) -> bool {
        use ConnectionType::*;
        match connection_type {
            LegacyIncoming => self.mark_legacy(public_key),
            LegacyOutgoing => {
                self.mark_legacy(public_key);
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
                    Err(e) => warn!(target: LOG_TARGET, "Listener failed to accept connection: {}", e),
                },
                // got a new command from the interface
                Some(command) = self.commands_from_interface.next() => match command {
                    // register new peer in manager or update its address if already there
                    // spawn a worker managing outgoing connection if the peer was not known
                    AddConnection(public_key, address) => {
                        // we add all the peers to the legacy manager so we don't lose the
                        // address, but only care about its opinion when it turns out we have to
                        // in particular the first time we add a peer we never know whether it
                        // requires legacy connecting, so we only attempt to connect to it if the
                        // new criterion is satisfied, otherwise we wait for it to connect to us
                        self.legacy_manager.add_peer(public_key.clone(), address.clone());
                        if self.manager.add_peer(public_key.clone(), address.clone()) {
                            self.spawn_new_outgoing(public_key, address, result_for_parent.clone());
                        };
                    },
                    // remove the peer from the manager all workers will be killed automatically, due to closed channels
                    DelConnection(public_key) => {
                        self.manager.remove_peer(&public_key);
                        self.legacy_manager.remove_peer(&public_key);
                        self.legacy_connected.remove(&public_key);
                    },
                    // pass the data to the manager
                    SendData(data, public_key) => {
                        match self.legacy_connected.contains(&public_key) {
                            true => match self.legacy_manager.send_to(&public_key, data) {
                                Ok(_) => trace!(target: LOG_TARGET, "Sending data to {} through legacy.", public_key),
                                Err(e) => trace!(target: LOG_TARGET, "Failed sending to {} through legacy: {}", public_key, e),
                            },
                            false => match self.manager.send_to(&public_key, data) {
                                Ok(_) => trace!(target: LOG_TARGET, "Sending data to {}.", public_key),
                                Err(e) => trace!(target: LOG_TARGET, "Failed sending to {}: {}", public_key, e),
                            },
                        }
                    },
                },
                // received information from a spawned worker managing a connection
                // check if we still want to be connected to the peer, and if so, spawn a new worker or actually add proper connection
                Some((public_key, maybe_data_for_network, connection_type)) = worker_results.next() => {
                    if self.check_for_legacy(&public_key, connection_type) {
                        match self.legacy_manager.peer_address(&public_key) {
                            Some(address) => self.spawn_new_outgoing(public_key.clone(), address, result_for_parent.clone()),
                            None => {
                                // We received a result from a worker we are no longer interested
                                // in.
                                self.legacy_connected.remove(&public_key);
                            },
                        }
                    }
                    use AddResult::*;
                    match maybe_data_for_network {
                        Some(data_for_network) => match self.add_connection(public_key.clone(), data_for_network, connection_type) {
                            Uninterested => warn!(target: LOG_TARGET, "Established connection with peer {} for unknown reasons.", public_key),
                            Added => info!(target: LOG_TARGET, "New connection with peer {}.", public_key),
                            Replaced => info!(target: LOG_TARGET, "Replaced connection with peer {}.", public_key),
                        },
                        None => if let Some(address) = self.peer_address(&public_key) {
                            self.spawn_new_outgoing(public_key, address, result_for_parent.clone());
                        }
                    }
                },
                // periodically reporting what we are trying to do
                _ = status_ticker.tick() => {
                    info!(target: LOG_TARGET, "Clique Network status: {}", self.manager.status_report());
                    debug!(target: LOG_TARGET, "Clique Network legacy status: {}", self.legacy_manager.status_report());
                }
                // received exit signal, stop the network
                // all workers will be killed automatically after the manager gets dropped
                _ = &mut exit => break,
            };
        }
    }
}
