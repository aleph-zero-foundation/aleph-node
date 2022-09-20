use std::{
    cmp,
    collections::{HashMap, HashSet},
    time::Duration,
};

use aleph_bft::Recipient;
use futures::{
    channel::{mpsc, oneshot},
    StreamExt,
};
use log::{debug, info, trace, warn};
use tokio::time::{self, Instant};

use crate::{
    crypto::{AuthorityPen, AuthorityVerifier},
    network::{
        manager::{
            Connections, Discovery, DiscoveryMessage, NetworkData, SessionHandler,
            SessionHandlerError,
        },
        ConnectionCommand, Data, DataCommand, Multiaddress, NetworkIdentity, Protocol,
    },
    MillisecsPerBlock, NodeIndex, SessionId, SessionPeriod, STATUS_REPORT_INTERVAL,
};

/// Commands for manipulating sessions, stopping them and starting both validator and non-validator
/// sessions.
pub enum SessionCommand<D: Data> {
    StartValidator(
        SessionId,
        AuthorityVerifier,
        NodeIndex,
        AuthorityPen,
        Option<oneshot::Sender<mpsc::UnboundedReceiver<D>>>,
    ),
    StartNonvalidator(SessionId, AuthorityVerifier),
    Stop(SessionId),
}

struct Session<D: Data, M: Multiaddress> {
    handler: SessionHandler<M>,
    discovery: Discovery<M>,
    data_for_user: Option<mpsc::UnboundedSender<D>>,
}

#[derive(Clone)]
struct PreValidatorSession {
    session_id: SessionId,
    verifier: AuthorityVerifier,
    node_id: NodeIndex,
    pen: AuthorityPen,
}

#[derive(Clone)]
struct PreNonvalidatorSession {
    session_id: SessionId,
    verifier: AuthorityVerifier,
}

#[derive(Clone)]
enum PreSession {
    Validator(PreValidatorSession),
    Nonvalidator(PreNonvalidatorSession),
}

impl PreSession {
    fn session_id(&self) -> SessionId {
        match self {
            Self::Validator(pre_session) => pre_session.session_id,
            Self::Nonvalidator(pre_session) => pre_session.session_id,
        }
    }
}

/// Configuration for the session manager service. Controls how often the maintenance and
/// rebroadcasts are triggerred. Also controls when maintenance starts.
pub struct Config {
    discovery_cooldown: Duration,
    maintenance_period: Duration,
    initial_delay: Duration,
}

impl Config {
    fn new(
        discovery_cooldown: Duration,
        maintenance_period: Duration,
        initial_delay: Duration,
    ) -> Self {
        Config {
            discovery_cooldown,
            maintenance_period,
            initial_delay,
        }
    }

    /// Returns a configuration that triggers maintenance about 5 times per session.
    pub fn with_session_period(
        session_period: &SessionPeriod,
        millisecs_per_block: &MillisecsPerBlock,
    ) -> Self {
        let discovery_cooldown =
            Duration::from_millis(millisecs_per_block.0 * session_period.0 as u64 / 5);
        let maintenance_period = discovery_cooldown / 2;
        let initial_delay = cmp::min(
            Duration::from_millis(millisecs_per_block.0 * 10),
            maintenance_period,
        );
        Config::new(discovery_cooldown, maintenance_period, initial_delay)
    }
}

type MessageForNetwork<D, M> = (NetworkData<D, M>, DataCommand<<M as Multiaddress>::PeerId>);

pub struct ServiceActions<D: Data, M: Multiaddress> {
    maybe_command: Option<ConnectionCommand<M>>,
    data: Vec<MessageForNetwork<D, M>>,
}

impl<D: Data, M: Multiaddress> ServiceActions<D, M> {
    fn noop() -> Self {
        ServiceActions {
            maybe_command: None,
            data: Vec::new(),
        }
    }
}

/// The connection manager service. It handles the abstraction over the network we build to support
/// separate sessions. This includes:
/// 1. Starting and ending specific sessions on user demand.
/// 2. Forwarding in-session user messages to the network using session handlers for address
///    translation.
/// 3. Handling network messages:
///    1. In-session messages are forwarded to the user.
///    2. Authentication messages forwarded to session handlers.
/// 4. Running periodic maintenance, mostly related to node discovery.
pub struct Service<NI: NetworkIdentity, D: Data> {
    network_identity: NI,
    connections: Connections<<NI::Multiaddress as Multiaddress>::PeerId>,
    sessions: HashMap<SessionId, Session<D, NI::Multiaddress>>,
    to_retry: Vec<(
        PreSession,
        Option<oneshot::Sender<mpsc::UnboundedReceiver<D>>>,
    )>,
    discovery_cooldown: Duration,
    maintenance_period: Duration,
    initial_delay: Duration,
}

impl<NI: NetworkIdentity, D: Data> Service<NI, D> {
    /// Create a new connection manager service.
    pub fn new(network_identity: NI, config: Config) -> Self {
        let Config {
            discovery_cooldown,
            maintenance_period,
            initial_delay,
        } = config;
        Service {
            network_identity,
            connections: Connections::new(),
            sessions: HashMap::new(),
            to_retry: Vec::new(),
            discovery_cooldown,
            maintenance_period,
            initial_delay,
        }
    }

    fn delete_reserved(
        to_remove: HashSet<NI::PeerId>,
    ) -> Option<ConnectionCommand<NI::Multiaddress>> {
        match to_remove.is_empty() {
            true => None,
            false => Some(ConnectionCommand::DelReserved(to_remove)),
        }
    }

    fn finish_session(
        &mut self,
        session_id: SessionId,
    ) -> Option<ConnectionCommand<NI::Multiaddress>> {
        self.sessions.remove(&session_id);
        self.to_retry
            .retain(|(pre_session, _)| pre_session.session_id() != session_id);
        Self::delete_reserved(self.connections.remove_session(session_id))
    }

    fn network_message(
        (message, command): (DiscoveryMessage<NI::Multiaddress>, DataCommand<NI::PeerId>),
    ) -> MessageForNetwork<D, NI::Multiaddress> {
        (NetworkData::Meta(message), command)
    }

    fn discover_authorities(
        &mut self,
        session_id: &SessionId,
    ) -> Vec<MessageForNetwork<D, NI::Multiaddress>> {
        if let Some(Session {
            handler, discovery, ..
        }) = self.sessions.get_mut(session_id)
        {
            discovery
                .discover_authorities(handler)
                .into_iter()
                .map(Self::network_message)
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Returns all the network messages that should be sent as part of discovery at this moment.
    pub fn discovery(&mut self) -> Vec<MessageForNetwork<D, NI::Multiaddress>> {
        let mut result = Vec::new();
        let sessions: Vec<_> = self.sessions.keys().cloned().collect();
        for session_id in sessions {
            result.append(&mut self.discover_authorities(&session_id));
        }
        result
    }

    fn addresses(&self) -> Vec<NI::Multiaddress> {
        let (addresses, peer_id) = self.network_identity.identity();
        debug!(target: "aleph-network", "Got addresses:\n{:?}\n and peer_id:{:?}", addresses, peer_id);
        addresses
            .into_iter()
            .filter_map(|address| address.add_matching_peer_id(peer_id))
            .collect()
    }

    async fn start_validator_session(
        &mut self,
        pre_session: PreValidatorSession,
        addresses: Vec<NI::Multiaddress>,
    ) -> Result<
        (
            Vec<MessageForNetwork<D, NI::Multiaddress>>,
            mpsc::UnboundedReceiver<D>,
        ),
        SessionHandlerError,
    > {
        let PreValidatorSession {
            session_id,
            verifier,
            node_id,
            pen,
        } = pre_session;
        let handler =
            SessionHandler::new(Some((node_id, pen)), verifier, session_id, addresses).await?;
        let discovery = Discovery::new(self.discovery_cooldown);
        let (data_for_user, data_from_network) = mpsc::unbounded();
        let data_for_user = Some(data_for_user);
        self.sessions.insert(
            session_id,
            Session {
                handler,
                discovery,
                data_for_user,
            },
        );
        Ok((self.discover_authorities(&session_id), data_from_network))
    }

    async fn update_validator_session(
        &mut self,
        pre_session: PreValidatorSession,
    ) -> Result<
        (
            ServiceActions<D, NI::Multiaddress>,
            mpsc::UnboundedReceiver<D>,
        ),
        SessionHandlerError,
    > {
        let addresses = self.addresses();
        let session = match self.sessions.get_mut(&pre_session.session_id) {
            Some(session) => session,
            None => {
                let (data, data_from_network) =
                    self.start_validator_session(pre_session, addresses).await?;
                return Ok((
                    ServiceActions {
                        maybe_command: None,
                        data,
                    },
                    data_from_network,
                ));
            }
        };
        let PreValidatorSession {
            session_id,
            verifier,
            node_id,
            pen,
        } = pre_session;
        let peers_to_stay = session
            .handler
            .update(Some((node_id, pen)), verifier, addresses)
            .await?
            .iter()
            .flat_map(|address| address.get_peer_id())
            .collect();
        let maybe_command = Self::delete_reserved(
            self.connections
                .remove_session(session_id)
                .difference(&peers_to_stay)
                .cloned()
                .collect(),
        );
        let (data_for_user, data_from_network) = mpsc::unbounded();
        session.data_for_user = Some(data_for_user);
        self.connections.add_peers(session_id, peers_to_stay);
        Ok((
            ServiceActions {
                maybe_command,
                data: self.discover_authorities(&session_id),
            },
            data_from_network,
        ))
    }

    async fn handle_validator_presession(
        &mut self,
        pre_session: PreValidatorSession,
        result_for_user: Option<oneshot::Sender<mpsc::UnboundedReceiver<D>>>,
    ) -> Result<ServiceActions<D, NI::Multiaddress>, SessionHandlerError> {
        match self.update_validator_session(pre_session.clone()).await {
            Ok((actions, data_from_network)) => {
                if let Some(result_for_user) = result_for_user {
                    if result_for_user.send(data_from_network).is_err() {
                        warn!(target: "aleph-network", "Failed to send started session.")
                    }
                }
                Ok(actions)
            }
            Err(e) => {
                self.to_retry
                    .push((PreSession::Validator(pre_session), result_for_user));
                Err(e)
            }
        }
    }

    async fn start_nonvalidator_session(
        &mut self,
        pre_session: PreNonvalidatorSession,
        addresses: Vec<NI::Multiaddress>,
    ) -> Result<(), SessionHandlerError> {
        let PreNonvalidatorSession {
            session_id,
            verifier,
        } = pre_session;
        let handler = SessionHandler::new(None, verifier, session_id, addresses).await?;
        let discovery = Discovery::new(self.discovery_cooldown);
        self.sessions.insert(
            session_id,
            Session {
                handler,
                discovery,
                data_for_user: None,
            },
        );
        Ok(())
    }

    async fn update_nonvalidator_session(
        &mut self,
        pre_session: PreNonvalidatorSession,
    ) -> Result<(), SessionHandlerError> {
        let addresses = self.addresses();
        let session = match self.sessions.get_mut(&pre_session.session_id) {
            Some(session) => session,
            None => {
                return self
                    .start_nonvalidator_session(pre_session, addresses)
                    .await;
            }
        };
        session
            .handler
            .update(None, pre_session.verifier, addresses)
            .await?;
        Ok(())
    }

    async fn handle_nonvalidator_presession(
        &mut self,
        pre_session: PreNonvalidatorSession,
    ) -> Result<(), SessionHandlerError> {
        self.update_nonvalidator_session(pre_session.clone())
            .await
            .map_err(|e| {
                self.to_retry
                    .push((PreSession::Nonvalidator(pre_session), None));
                e
            })
    }

    /// Handle a session command.
    /// Returns a command possibly changing what we should stay connected to and a list of data to
    /// be sent over the network.
    pub async fn on_command(
        &mut self,
        command: SessionCommand<D>,
    ) -> Result<ServiceActions<D, NI::Multiaddress>, SessionHandlerError> {
        use SessionCommand::*;
        match command {
            StartValidator(session_id, verifier, node_id, pen, result_for_user) => {
                let pre_session = PreValidatorSession {
                    session_id,
                    verifier,
                    node_id,
                    pen,
                };
                self.handle_validator_presession(pre_session, result_for_user)
                    .await
            }
            StartNonvalidator(session_id, verifier) => {
                let pre_session = PreNonvalidatorSession {
                    session_id,
                    verifier,
                };
                self.handle_nonvalidator_presession(pre_session).await?;
                Ok(ServiceActions::noop())
            }
            Stop(session_id) => Ok(ServiceActions {
                maybe_command: self.finish_session(session_id),
                data: Vec::new(),
            }),
        }
    }

    /// Handle a user request for sending data.
    /// Returns a list of data to be sent over the network.
    pub fn on_user_message(
        &self,
        message: D,
        session_id: SessionId,
        recipient: Recipient,
    ) -> Vec<MessageForNetwork<D, NI::Multiaddress>> {
        if let Some(handler) = self
            .sessions
            .get(&session_id)
            .map(|session| &session.handler)
        {
            let to_send = NetworkData::Data(message, session_id);
            match recipient {
                Recipient::Everyone => (0..handler.node_count().0)
                    .map(NodeIndex)
                    .flat_map(|node_id| handler.peer_id(&node_id))
                    .map(|peer_id| {
                        (
                            to_send.clone(),
                            DataCommand::SendTo(peer_id, Protocol::Validator),
                        )
                    })
                    .collect(),
                Recipient::Node(node_id) => handler
                    .peer_id(&node_id)
                    .into_iter()
                    .map(|peer_id| {
                        (
                            to_send.clone(),
                            DataCommand::SendTo(peer_id, Protocol::Validator),
                        )
                    })
                    .collect(),
            }
        } else {
            Vec::new()
        }
    }

    /// Handle a discovery message.
    /// Returns a command possibly changing what we should stay connected to and a list of data to
    /// be sent over the network.
    pub fn on_discovery_message(
        &mut self,
        message: DiscoveryMessage<NI::Multiaddress>,
    ) -> ServiceActions<D, NI::Multiaddress> {
        let session_id = message.session_id();
        match self.sessions.get_mut(&session_id) {
            Some(Session {
                handler, discovery, ..
            }) => {
                let (addresses, responses) = discovery.handle_message(message, handler);
                let maybe_command = match !addresses.is_empty() && handler.is_validator() {
                    true => {
                        debug!(target: "aleph-network", "Adding addresses for session {:?} to reserved: {:?}", session_id, addresses);
                        self.connections.add_peers(
                            session_id,
                            addresses.iter().flat_map(|address| address.get_peer_id()),
                        );
                        Some(ConnectionCommand::AddReserved(
                            addresses.into_iter().collect(),
                        ))
                    }
                    false => None,
                };
                ServiceActions {
                    maybe_command,
                    data: responses.into_iter().map(Self::network_message).collect(),
                }
            }
            None => {
                debug!(target: "aleph-network", "Received message from unknown session: {:?}", message);
                ServiceActions::noop()
            }
        }
    }

    /// Sends the data to the identified session.
    pub fn send_session_data(&self, session_id: &SessionId, data: D) -> Result<(), Error> {
        match self
            .sessions
            .get(session_id)
            .and_then(|session| session.data_for_user.as_ref())
        {
            Some(data_for_user) => data_for_user
                .unbounded_send(data)
                .map_err(|_| Error::UserSend),
            None => Err(Error::NoSession),
        }
    }

    /// Retries starting a validator session the user requested, but which failed to start
    /// initially. Mostly useful when the network was not yet aware of its own address at time of
    /// the request.
    pub async fn retry_session_start(
        &mut self,
    ) -> Result<ServiceActions<D, NI::Multiaddress>, SessionHandlerError> {
        let (pre_session, result_for_user) = match self.to_retry.pop() {
            Some(to_retry) => to_retry,
            None => return Ok(ServiceActions::noop()),
        };
        match pre_session {
            PreSession::Validator(pre_session) => {
                self.handle_validator_presession(pre_session, result_for_user)
                    .await
            }
            PreSession::Nonvalidator(pre_session) => {
                self.handle_nonvalidator_presession(pre_session).await?;
                Ok(ServiceActions::noop())
            }
        }
    }

    pub fn status_report(&self) {
        let mut status = String::from("Connection Manager status report: ");

        let mut authenticated: Vec<_> = self
            .sessions
            .iter()
            .filter(|(_, session)| session.handler.authentication().is_some())
            .map(|(session_id, session)| {
                let mut peers = session
                    .handler
                    .peers()
                    .into_iter()
                    .map(|(node_id, peer_id)| (node_id.0, peer_id))
                    .collect::<Vec<_>>();
                peers.sort_by(|x, y| x.0.cmp(&y.0));
                (session_id.0, session.handler.node_count().0, peers)
            })
            .collect();
        authenticated.sort_by(|x, y| x.0.cmp(&y.0));
        if !authenticated.is_empty() {
            let authenticated_status = authenticated
                .iter()
                .map(|(session_id, node_count, peers)| {
                    let peer_ids = peers
                        .iter()
                        .map(|(node_id, peer_id)| format!("{:?}: {}", node_id, peer_id,))
                        .collect::<Vec<_>>()
                        .join(", ");

                    format!(
                        "{:?}: {}/{} {{{}}}",
                        session_id,
                        peers.len() + 1,
                        node_count,
                        peer_ids
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            status.push_str(&format!(
                "authenticated authorities: {}; ",
                authenticated_status
            ));
        }

        let mut missing: Vec<_> = self
            .sessions
            .iter()
            .filter(|(_, session)| session.handler.authentication().is_some())
            .map(|(session_id, session)| {
                (
                    session_id.0,
                    session
                        .handler
                        .missing_nodes()
                        .iter()
                        .map(|id| id.0)
                        .collect::<Vec<_>>(),
                )
            })
            .filter(|(_, missing)| !missing.is_empty())
            .collect();
        missing.sort_by(|x, y| x.0.cmp(&y.0));
        if !missing.is_empty() {
            let missing_status = missing
                .iter()
                .map(|(session_id, missing)| format!("{:?}: {:?}", session_id, missing))
                .collect::<Vec<_>>()
                .join(", ");
            status.push_str(&format!("missing authorities: {}; ", missing_status));
        }

        if !authenticated.is_empty() || !missing.is_empty() {
            info!(target: "aleph-network", "{}", status);
        }
    }
}

/// Input/output interface for the connectiona manager service.
pub struct IO<D: Data, M: Multiaddress> {
    commands_for_network: mpsc::UnboundedSender<ConnectionCommand<M>>,
    messages_for_network: mpsc::UnboundedSender<MessageForNetwork<D, M>>,
    commands_from_user: mpsc::UnboundedReceiver<SessionCommand<D>>,
    messages_from_user: mpsc::UnboundedReceiver<(D, SessionId, Recipient)>,
    messages_from_network: mpsc::UnboundedReceiver<NetworkData<D, M>>,
}

/// Errors that can happen during the network service operations.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    NetworkSend,
    CommandSend,
    /// Should never be fatal.
    UserSend,
    /// Should never be fatal.
    NoSession,
    CommandsChannel,
    MessageChannel,
    NetworkChannel,
}

impl<D: Data, M: Multiaddress> IO<D, M> {
    pub fn new(
        commands_for_network: mpsc::UnboundedSender<ConnectionCommand<M>>,
        messages_for_network: mpsc::UnboundedSender<MessageForNetwork<D, M>>,
        commands_from_user: mpsc::UnboundedReceiver<SessionCommand<D>>,
        messages_from_user: mpsc::UnboundedReceiver<(D, SessionId, Recipient)>,
        messages_from_network: mpsc::UnboundedReceiver<NetworkData<D, M>>,
    ) -> IO<D, M> {
        IO {
            commands_for_network,
            messages_for_network,
            commands_from_user,
            messages_from_user,
            messages_from_network,
        }
    }

    fn send_data(&self, to_send: MessageForNetwork<D, M>) -> Result<(), Error> {
        self.messages_for_network
            .unbounded_send(to_send)
            .map_err(|_| Error::NetworkSend)
    }

    fn send_command(&self, to_send: ConnectionCommand<M>) -> Result<(), Error> {
        self.commands_for_network
            .unbounded_send(to_send)
            .map_err(|_| Error::CommandSend)
    }

    fn send(
        &self,
        ServiceActions {
            maybe_command,
            data,
        }: ServiceActions<D, M>,
    ) -> Result<(), Error> {
        if let Some(command) = maybe_command {
            self.send_command(command)?;
        }
        for data_to_send in data {
            self.send_data(data_to_send)?;
        }
        Ok(())
    }

    fn on_network_message<NI: NetworkIdentity<Multiaddress = M, PeerId = M::PeerId>>(
        &self,
        service: &mut Service<NI, D>,
        message: NetworkData<D, M>,
    ) -> Result<(), Error> {
        use NetworkData::*;
        match message {
            Meta(message) => self.send(service.on_discovery_message(message)),
            Data(data, session_id) => service.send_session_data(&session_id, data),
        }
    }

    /// Run the connection manager service with this IO.
    pub async fn run<NI: NetworkIdentity<Multiaddress = M, PeerId = M::PeerId>>(
        mut self,
        mut service: Service<NI, D>,
    ) -> Result<(), Error> {
        // Initial delay is needed so that Network is fully set up and we received some first discovery broadcasts from other nodes.
        // Otherwise this might cause first maintenance never working, as it happens before first broadcasts.
        let mut maintenance = time::interval_at(
            Instant::now() + service.initial_delay,
            service.maintenance_period,
        );

        let mut status_ticker = time::interval(STATUS_REPORT_INTERVAL);
        loop {
            trace!(target: "aleph-network", "Manager Loop started a next iteration");
            tokio::select! {
                maybe_command = self.commands_from_user.next() => {
                    trace!(target: "aleph-network", "Manager received a command from user");
                    match maybe_command {
                        Some(command) => match service.on_command(command).await {
                            Ok(to_send) => self.send(to_send)?,
                            Err(e) => warn!(target: "aleph-network", "Failed to update handler: {:?}", e),
                        },
                        None => return Err(Error::CommandsChannel),
                    }
                },
                maybe_message = self.messages_from_user.next() => {
                    trace!(target: "aleph-network", "Manager received a message from user");
                    match maybe_message {
                        Some((message, session_id, recipient)) => for message in service.on_user_message(message, session_id, recipient) {
                            self.send_data(message)?;
                        },
                        None => return Err(Error::MessageChannel),
                    }
                },
                maybe_message = self.messages_from_network.next() => {
                    trace!(target: "aleph-network", "Manager received a message from network");
                    match maybe_message {
                        Some(message) => if let Err(e) = self.on_network_message(&mut service, message) {
                            match e {
                                Error::UserSend => trace!(target: "aleph-network", "Failed to send to user in session."),
                                Error::NoSession => trace!(target: "aleph-network", "Received message for unknown session."),
                                _ => return Err(e),
                            }
                        },
                        None => return Err(Error::NetworkChannel),
                    }
                },
                _ = maintenance.tick() => {
                    debug!(target: "aleph-network", "Manager starts maintenence");
                    match service.retry_session_start().await {
                        Ok(to_send) => self.send(to_send)?,
                        Err(e) => warn!(target: "aleph-network", "Retry failed to update handler: {:?}", e),
                    }
                    for to_send in service.discovery() {
                        self.send_data(to_send)?;
                    }
                },
                _ = status_ticker.tick() => {
                    service.status_report();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use aleph_bft::Recipient;
    use futures::{channel::oneshot, StreamExt};

    use super::{Config, Error, Service, ServiceActions, SessionCommand};
    use crate::{
        network::{
            manager::{DiscoveryMessage, NetworkData},
            mock::{crypto_basics, MockNetworkIdentity},
            ConnectionCommand, DataCommand, Protocol,
        },
        SessionId,
    };

    const NUM_NODES: usize = 7;
    const MAINTENANCE_PERIOD: Duration = Duration::from_secs(120);
    const DISCOVERY_PERIOD: Duration = Duration::from_secs(60);
    const INITIAL_DELAY: Duration = Duration::from_secs(5);

    fn build() -> Service<MockNetworkIdentity, i32> {
        Service::new(
            MockNetworkIdentity::new(),
            Config::new(MAINTENANCE_PERIOD, DISCOVERY_PERIOD, INITIAL_DELAY),
        )
    }

    #[tokio::test]
    async fn starts_nonvalidator_session() {
        let mut service = build();
        let (_, verifier) = crypto_basics(NUM_NODES).await;
        let session_id = SessionId(43);
        let ServiceActions {
            maybe_command,
            data,
        } = service
            .on_command(SessionCommand::StartNonvalidator(session_id, verifier))
            .await
            .unwrap();
        assert!(maybe_command.is_none());
        assert!(data.is_empty());
        assert_eq!(
            service.send_session_data(&session_id, -43),
            Err(Error::NoSession)
        );
    }

    #[tokio::test]
    async fn starts_validator_session() {
        let mut service = build();
        let (validator_data, verifier) = crypto_basics(NUM_NODES).await;
        let (node_id, pen) = validator_data[0].clone();
        let session_id = SessionId(43);
        let (result_for_user, result_from_service) = oneshot::channel();
        let ServiceActions {
            maybe_command,
            data,
        } = service
            .on_command(SessionCommand::StartValidator(
                session_id,
                verifier,
                node_id,
                pen,
                Some(result_for_user),
            ))
            .await
            .unwrap();
        assert!(maybe_command.is_none());
        assert_eq!(data.len(), 1);
        assert!(data
            .iter()
            .all(|(_, command)| command == &DataCommand::Broadcast));
        let _data_from_network = result_from_service.await.unwrap();
        assert_eq!(service.send_session_data(&session_id, -43), Ok(()));
    }

    #[tokio::test]
    async fn stops_session() {
        let mut service = build();
        let (validator_data, verifier) = crypto_basics(NUM_NODES).await;
        let (node_id, pen) = validator_data[0].clone();
        let session_id = SessionId(43);
        let (result_for_user, result_from_service) = oneshot::channel();
        let ServiceActions {
            maybe_command,
            data,
        } = service
            .on_command(SessionCommand::StartValidator(
                session_id,
                verifier,
                node_id,
                pen,
                Some(result_for_user),
            ))
            .await
            .unwrap();
        assert!(maybe_command.is_none());
        assert_eq!(data.len(), 1);
        assert!(data
            .iter()
            .all(|(_, command)| command == &DataCommand::Broadcast));
        assert_eq!(service.send_session_data(&session_id, -43), Ok(()));
        let mut data_from_network = result_from_service.await.unwrap();
        assert_eq!(data_from_network.next().await, Some(-43));
        let ServiceActions {
            maybe_command,
            data,
        } = service
            .on_command(SessionCommand::Stop(session_id))
            .await
            .unwrap();
        assert!(maybe_command.is_none());
        assert!(data.is_empty());
        assert_eq!(
            service.send_session_data(&session_id, -43),
            Err(Error::NoSession)
        );
        assert!(data_from_network.next().await.is_none());
    }

    #[tokio::test]
    async fn handles_broadcast() {
        let mut service = build();
        let (validator_data, verifier) = crypto_basics(NUM_NODES).await;
        let (node_id, pen) = validator_data[0].clone();
        let session_id = SessionId(43);
        service
            .on_command(SessionCommand::StartValidator(
                session_id,
                verifier.clone(),
                node_id,
                pen,
                None,
            ))
            .await
            .unwrap();
        let mut other_service = build();
        let (node_id, pen) = validator_data[1].clone();
        let ServiceActions { data, .. } = other_service
            .on_command(SessionCommand::StartValidator(
                session_id, verifier, node_id, pen, None,
            ))
            .await
            .unwrap();
        let broadcast = match data[0].clone() {
            (NetworkData::Meta(broadcast), DataCommand::Broadcast) => broadcast,
            _ => panic!("Expected discovery massage broadcast, got: {:?}", data[0]),
        };
        let addresses = match &broadcast {
            DiscoveryMessage::AuthenticationBroadcast((auth_data, _)) => auth_data.addresses(),
            _ => panic!("Expected an authentication broadcast, got {:?}", broadcast),
        };
        let ServiceActions {
            maybe_command,
            data,
        } = service.on_discovery_message(broadcast);
        assert_eq!(
            maybe_command,
            Some(ConnectionCommand::AddReserved(
                addresses.into_iter().collect()
            ))
        );
        assert_eq!(data.len(), 2);
        assert!(data
            .iter()
            .any(|(_, command)| command == &DataCommand::Broadcast));
        assert!(data
            .iter()
            .any(|(_, command)| matches!(command, &DataCommand::SendTo(_, _))));
    }

    #[tokio::test]
    async fn sends_user_data() {
        let mut service = build();
        let (validator_data, verifier) = crypto_basics(NUM_NODES).await;
        let (node_id, pen) = validator_data[0].clone();
        let session_id = SessionId(43);
        service
            .on_command(SessionCommand::StartValidator(
                session_id,
                verifier.clone(),
                node_id,
                pen,
                None,
            ))
            .await
            .unwrap();
        let mut other_service = build();
        let (node_id, pen) = validator_data[1].clone();
        let ServiceActions { data, .. } = other_service
            .on_command(SessionCommand::StartValidator(
                session_id, verifier, node_id, pen, None,
            ))
            .await
            .unwrap();
        let broadcast = match data[0].clone() {
            (NetworkData::Meta(broadcast), DataCommand::Broadcast) => broadcast,
            _ => panic!("Expected discovery massage broadcast, got: {:?}", data[0]),
        };
        service.on_discovery_message(broadcast);
        let messages = service.on_user_message(2137, session_id, Recipient::Everyone);
        assert_eq!(messages.len(), 1);
        let (network_data, data_command) = &messages[0];
        assert!(matches!(
            data_command,
            DataCommand::SendTo(_, Protocol::Validator)
        ));
        assert_eq!(network_data, &NetworkData::Data(2137, session_id));
    }
}
