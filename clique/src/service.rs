use std::{fmt::Debug, pin::Pin, time::Duration};

use futures::{
    channel::{mpsc, oneshot},
    Future, StreamExt,
};
use log::{info, trace, warn};
use substrate_prometheus_endpoint::Registry;
use tokio::time;

use crate::{
    incoming::incoming,
    manager::{AddResult, Manager},
    metrics::Metrics,
    outgoing::outgoing,
    protocols::ResultForService,
    Data, Dialer, Listener, Network, PeerId, PublicKey, SecretKey, LOG_TARGET,
};

const STATUS_REPORT_INTERVAL: Duration = Duration::from_secs(20);

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

/// Trait abstracting spawning tasks
pub trait SpawnHandleT {
    /// Run task
    fn spawn(&self, name: &'static str, task: impl Future<Output = ()> + Send + 'static);

    /// Run an essential task
    fn spawn_essential(
        &self,
        name: &'static str,
        task: impl Future<Output = ()> + Send + 'static,
    ) -> Pin<Box<dyn Future<Output = Result<(), ()>> + Send>>;
}

/// A service that has to be run for the clique network to work.
pub struct Service<SK: SecretKey, D: Data, A: Data, ND: Dialer<A>, NL: Listener, SH: SpawnHandleT>
where
    SK::PublicKey: PeerId,
{
    commands_from_interface: mpsc::UnboundedReceiver<ServiceCommand<SK::PublicKey, D, A>>,
    next_to_interface: mpsc::UnboundedSender<D>,
    manager: Manager<SK::PublicKey, A, D>,
    dialer: ND,
    listener: NL,
    spawn_handle: SH,
    secret_key: SK,
    metrics: Metrics,
}

impl<SK: SecretKey, D: Data, A: Data + Debug, ND: Dialer<A>, NL: Listener, SH: SpawnHandleT>
    Service<SK, D, A, ND, NL, SH>
where
    SK::PublicKey: PeerId,
{
    /// Create a new clique network service plus an interface for interacting with it.
    pub fn new(
        dialer: ND,
        listener: NL,
        secret_key: SK,
        spawn_handle: SH,
        metrics_registry: Option<Registry>,
    ) -> (Self, impl Network<SK::PublicKey, A, D>) {
        // Channel for sending commands between the service and interface
        let (commands_for_service, commands_from_interface) = mpsc::unbounded();
        // Channel for receiving data from the network
        let (next_to_interface, next_from_service) = mpsc::unbounded();
        let metrics = match Metrics::new(metrics_registry) {
            Ok(metrics) => metrics,
            Err(e) => {
                warn!(target: LOG_TARGET, "Failed to create metrics: {}", e);
                Metrics::noop()
            }
        };
        (
            Self {
                commands_from_interface,
                next_to_interface,
                manager: Manager::new(secret_key.public_key(), metrics.clone()),
                dialer,
                listener,
                spawn_handle,
                secret_key,
                metrics,
            },
            ServiceInterface {
                commands_for_service,
                next_from_service,
            },
        )
    }

    fn spawn_new_outgoing(
        &mut self,
        public_key: SK::PublicKey,
        address: A,
        result_for_parent: mpsc::UnboundedSender<ResultForService<SK::PublicKey, D>>,
    ) {
        let secret_key = self.secret_key.clone();
        let dialer = self.dialer.clone();
        let next_to_interface = self.next_to_interface.clone();
        let metrics = self.metrics.clone();
        self.spawn_handle
            .spawn("aleph/clique_network_outgoing", async move {
                outgoing(
                    secret_key,
                    public_key,
                    dialer,
                    address,
                    result_for_parent,
                    next_to_interface,
                    metrics,
                )
                .await;
            });
    }

    fn spawn_new_incoming(
        &self,
        stream: NL::Connection,
        result_for_parent: mpsc::UnboundedSender<ResultForService<SK::PublicKey, D>>,
        authorization_requests_sender: mpsc::UnboundedSender<(
            SK::PublicKey,
            oneshot::Sender<bool>,
        )>,
    ) {
        let secret_key = self.secret_key.clone();
        let next_to_interface = self.next_to_interface.clone();
        let metrics = self.metrics.clone();
        self.spawn_handle
            .spawn("aleph/clique_network_incoming", async move {
                incoming(
                    secret_key,
                    stream,
                    result_for_parent,
                    next_to_interface,
                    authorization_requests_sender,
                    metrics,
                )
                .await;
            });
    }

    fn peer_address(&self, public_key: &SK::PublicKey) -> Option<A> {
        self.manager.peer_address(public_key)
    }

    fn add_connection(
        &mut self,
        public_key: SK::PublicKey,
        data_for_network: mpsc::UnboundedSender<D>,
    ) -> AddResult {
        self.manager.add_connection(public_key, data_for_network)
    }

    /// Run the service until a signal from exit.
    pub async fn run(mut self, mut exit: oneshot::Receiver<()>) {
        let mut status_ticker = time::interval(STATUS_REPORT_INTERVAL);
        let (result_for_parent, mut worker_results) = mpsc::unbounded();
        let (authorization_requests_sender, mut authorization_requests) = mpsc::unbounded();
        use ServiceCommand::*;
        loop {
            tokio::select! {
                // got new incoming connection from the listener - spawn an incoming worker
                maybe_stream = self.listener.accept() => match maybe_stream {
                    Ok(stream) => self.spawn_new_incoming(stream, result_for_parent.clone(), authorization_requests_sender.clone()),
                    Err(e) => warn!(target: LOG_TARGET, "Listener failed to accept connection: {}", e),
                },
                // got a new command from the interface
                Some(command) = self.commands_from_interface.next() => match command {
                    // register new peer in manager or update its address if already there
                    // spawn a worker managing outgoing connection if the peer was not known
                    AddConnection(public_key, address) => {
                        if self.manager.add_peer(public_key.clone(), address.clone()) {
                            self.spawn_new_outgoing(public_key, address, result_for_parent.clone());
                        };
                    },
                    // remove the peer from the manager all workers will be killed automatically, due to closed channels
                    DelConnection(public_key) => {
                        self.manager.remove_peer(&public_key);
                    },
                    // pass the data to the manager
                    SendData(data, public_key) => {
                        match self.manager.send_to(&public_key, data) {
                                Ok(_) => trace!(target: LOG_TARGET, "Sending data to {}.", public_key),
                                Err(e) => trace!(target: LOG_TARGET, "Failed sending to {}: {}", public_key, e),
                            }
                    }
                },
                Some((public_key, response_channel)) = authorization_requests.next() => {
                    let authorization_result = self.manager.is_authorized(&public_key);
                    if response_channel.send(authorization_result).is_err() {
                        warn!(target: LOG_TARGET, "Other side of the Authorization Service is already closed.");
                    }
                },
                // received information from a spawned worker managing a connection
                // check if we still want to be connected to the peer, and if so, spawn a new worker or actually add proper connection
                Some((public_key, maybe_data_for_network)) = worker_results.next() => {
                    use AddResult::*;
                    match maybe_data_for_network {
                        Some(data_for_network) => match self.add_connection(public_key.clone(), data_for_network) {
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
                }
                // received exit signal, stop the network
                // all workers will be killed automatically after the manager gets dropped
                _ = &mut exit => break,
            };
        }
    }
}
