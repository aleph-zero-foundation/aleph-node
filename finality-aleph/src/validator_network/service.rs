use std::fmt::Debug;

use aleph_primitives::AuthorityId;
use futures::{
    channel::{mpsc, oneshot},
    StreamExt,
};
use log::{info, trace, warn};
use tokio::time;

use crate::{
    crypto::AuthorityPen,
    validator_network::{
        incoming::incoming,
        manager::{AddResult, Manager},
        outgoing::outgoing,
        protocols::ResultForService,
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
pub struct Service<D: Data, A: Data + Debug, ND: Dialer<A>, NL: Listener> {
    commands_from_interface: mpsc::UnboundedReceiver<ServiceCommand<D, A>>,
    next_to_interface: mpsc::UnboundedSender<D>,
    manager: Manager<A, D>,
    dialer: ND,
    listener: NL,
    spawn_handle: SpawnTaskHandle,
    authority_pen: AuthorityPen,
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
                manager: Manager::new(),
                dialer,
                listener,
                spawn_handle,
                authority_pen,
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
        // This isn't really currently used, but soon will be.
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

    /// Run the service until a signal from exit.
    pub async fn run(mut self, mut exit: oneshot::Receiver<()>) {
        let mut status_ticker = time::interval(STATUS_REPORT_INTERVAL);
        // channel used to receive tuple (peer_id, exit_handle) from a spawned worker
        // that has just established an incoming connection
        // exit_handle may be used to kill the worker later
        let (incoming_result_for_parent, mut incoming_workers) = mpsc::unbounded();
        // channel used to receive information about failure from a spawned worker
        // that managed an outgoing connection
        // the received peer_id can be used to spawn another worker
        let (outgoing_result_for_parent, mut outgoing_workers) = mpsc::unbounded();
        use ServiceCommand::*;
        loop {
            tokio::select! {
                // got new incoming connection from the listener - spawn an incoming worker
                maybe_stream = self.listener.accept() => match maybe_stream {
                    Ok(stream) => self.spawn_new_incoming(stream, incoming_result_for_parent.clone()),
                    Err(e) => warn!(target: "validator-network", "Listener failed to accept connection: {}", e),
                },
                // got a new command from the interface
                Some(command) = self.commands_from_interface.next() => match command {
                    // register new peer in manager or update its list of addresses if already there
                    // spawn a worker managing outgoing connection if the peer was not known
                    AddConnection(peer_id, addresses) => {
                        if self.manager.add_peer(peer_id.clone(), addresses.clone()) {
                            self.spawn_new_outgoing(peer_id, addresses, outgoing_result_for_parent.clone());
                        };
                    },
                    // remove the peer from the manager all workers will be killed automatically, due to closed channels
                    DelConnection(peer_id) => {
                        self.manager.remove_peer(&peer_id);
                    },
                    // pass the data to the manager
                    SendData(data, peer_id) => {
                        match self.manager.send_to(&peer_id, data) {
                            Ok(_) => trace!(target: "validator-network", "Sending data to {}.", peer_id),
                            Err(e) => trace!(target: "validator-network", "Failed sending to {}: {}", peer_id, e),
                        }
                    },
                },
                // received tuple (peer_id, exit_handle) from a spawned worker
                // that has just established an incoming connection
                // pass the tuple to the manager to register the connection
                // the manager will be responsible for killing the worker if necessary
                Some((peer_id, Some(exit), _)) = incoming_workers.next() => {
                    use AddResult::*;
                    match self.manager.add_incoming(peer_id.clone(), exit) {
                        Uninterested => info!(target: "validator-network", "Peer {} connected to us despite out lack of interest.", peer_id),
                        Added => info!(target: "validator-network", "New incoming connection for peer {}.", peer_id),
                        Replaced => info!(target: "validator-network", "Replaced incoming connection for peer {}.", peer_id),
                    }
                },
                // received information from a spawned worker managing an outgoing connection
                // check if we still want to be connected to the peer, and if so, spawn a new worker or actually add proper connection
                Some((peer_id, maybe_data_for_network, _)) = outgoing_workers.next() => {
                    use AddResult::*;
                    if let Some(addresses) = self.manager.peer_addresses(&peer_id) {
                        match maybe_data_for_network {
                            Some(data_for_network) => match self.manager.add_outgoing(peer_id.clone(), data_for_network) {
                                Uninterested => warn!(target: "validator-network", "We connected to peer {} for unknown reasons.", peer_id),
                                Added => info!(target: "validator-network", "New outgoing connection to peer {}.", peer_id),
                                Replaced => info!(target: "validator-network", "Replaced outgoing connection to peer {}.", peer_id),
                            },
                            None => self.spawn_new_outgoing(peer_id, addresses, outgoing_result_for_parent.clone()),
                        }
                    };
                },
                // periodically reporting what we are trying to do
                _ = status_ticker.tick() => {
                    info!(target: "validator-network", "Validator Network status: {}", self.manager.status_report())
                }
                // received exit signal, stop the network
                // all workers will be killed automatically after the manager gets dropped
                _ = &mut exit => break,
            };
        }
    }
}
