use crate::{
    crypto::{AuthorityPen, AuthorityVerifier},
    network::{
        manager::{
            add_matching_peer_id, get_peer_id, Connections, Discovery, DiscoveryMessage, Multiaddr,
            NetworkData, SessionHandler, SessionHandlerError,
        },
        ConnectionCommand, Data, DataCommand, NetworkIdentity, PeerId, Protocol,
    },
    MillisecsPerBlock, NodeIndex, SessionId, SessionPeriod,
};
use aleph_bft::Recipient;
use futures::{
    channel::{mpsc, oneshot},
    StreamExt,
};
use log::{debug, trace, warn};
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};
use tokio::time::interval;

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

struct Session<D: Data> {
    handler: SessionHandler,
    discovery: Discovery,
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
/// rebroadcasts are triggerred.
pub struct Config {
    discovery_cooldown: Duration,
    maintenance_period: Duration,
}

impl Config {
    fn new(discovery_cooldown: Duration, maintenance_period: Duration) -> Self {
        Config {
            discovery_cooldown,
            maintenance_period,
        }
    }

    /// Returns a configuration that triggers maintenance about 5 times per session.
    pub fn with_session_period(
        session_period: &SessionPeriod,
        millisecs_per_block: &MillisecsPerBlock,
    ) -> Self {
        let discovery_cooldown =
            Duration::from_millis(millisecs_per_block.0 * session_period.0 as u64 / 5);
        Config::new(discovery_cooldown, discovery_cooldown / 2)
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
    connections: Connections,
    sessions: HashMap<SessionId, Session<D>>,
    to_retry: Vec<(
        PreSession,
        Option<oneshot::Sender<mpsc::UnboundedReceiver<D>>>,
    )>,
    discovery_cooldown: Duration,
    maintenance_period: Duration,
}

impl<NI: NetworkIdentity, D: Data> Service<NI, D> {
    /// Create a new connection manager service.
    pub fn new(network_identity: NI, config: Config) -> Self {
        let Config {
            discovery_cooldown,
            maintenance_period,
        } = config;
        Service {
            network_identity,
            connections: Connections::new(),
            sessions: HashMap::new(),
            to_retry: Vec::new(),
            discovery_cooldown,
            maintenance_period,
        }
    }

    fn delete_reserved(to_remove: HashSet<PeerId>) -> Option<ConnectionCommand> {
        match to_remove.is_empty() {
            true => None,
            false => Some(ConnectionCommand::DelReserved(to_remove)),
        }
    }

    fn finish_session(&mut self, session_id: SessionId) -> Option<ConnectionCommand> {
        self.sessions.remove(&session_id);
        self.to_retry
            .retain(|(pre_session, _)| pre_session.session_id() != session_id);
        Self::delete_reserved(self.connections.remove_session(session_id))
    }

    fn network_message(
        (message, command): (DiscoveryMessage, DataCommand),
    ) -> (NetworkData<D>, DataCommand) {
        (NetworkData::Meta(message), command)
    }

    fn discover_authorities(
        &mut self,
        session_id: &SessionId,
    ) -> Vec<(NetworkData<D>, DataCommand)> {
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
    pub fn discovery(&mut self) -> Vec<(NetworkData<D>, DataCommand)> {
        let mut result = Vec::new();
        let sessions: Vec<_> = self.sessions.keys().cloned().collect();
        for session_id in sessions {
            result.append(&mut self.discover_authorities(&session_id));
        }
        result
    }

    fn addresses(&self) -> Vec<Multiaddr> {
        let (addresses, peer_id) = self.network_identity.identity();
        debug!(target: "aleph-network", "Got addresses:\n{:?}\n and peer_id:{:?}", addresses, peer_id);
        addresses
            .into_iter()
            .map(Multiaddr)
            .filter_map(|address| add_matching_peer_id(address, peer_id))
            .collect()
    }

    async fn start_validator_session(
        &mut self,
        pre_session: PreValidatorSession,
        addresses: Vec<Multiaddr>,
    ) -> Result<
        (
            Vec<(NetworkData<D>, DataCommand)>,
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
            Option<ConnectionCommand>,
            Vec<(NetworkData<D>, DataCommand)>,
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
                return Ok((None, data, data_from_network));
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
            .flat_map(get_peer_id)
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
            maybe_command,
            self.discover_authorities(&session_id),
            data_from_network,
        ))
    }

    async fn handle_validator_presession(
        &mut self,
        pre_session: PreValidatorSession,
        result_for_user: Option<oneshot::Sender<mpsc::UnboundedReceiver<D>>>,
    ) -> Result<
        (
            Option<ConnectionCommand>,
            Vec<(NetworkData<D>, DataCommand)>,
        ),
        SessionHandlerError,
    > {
        match self.update_validator_session(pre_session.clone()).await {
            Ok((maybe_command, data, data_from_network)) => {
                if let Some(result_for_user) = result_for_user {
                    if result_for_user.send(data_from_network).is_err() {
                        warn!(target: "aleph-network", "Failed to send started session.")
                    }
                }
                Ok((maybe_command, data))
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
        addresses: Vec<Multiaddr>,
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
    ) -> Result<
        (
            Option<ConnectionCommand>,
            Vec<(NetworkData<D>, DataCommand)>,
        ),
        SessionHandlerError,
    > {
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
                Ok((None, Vec::new()))
            }
            Stop(session_id) => Ok((self.finish_session(session_id), Vec::new())),
        }
    }

    /// Handle a user request for sending data.
    /// Returns a list of data to be sent over the network.
    pub fn on_user_message(
        &self,
        message: D,
        session_id: SessionId,
        recipient: Recipient,
    ) -> Vec<(NetworkData<D>, DataCommand)> {
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
        message: DiscoveryMessage,
    ) -> (
        Option<ConnectionCommand>,
        Vec<(NetworkData<D>, DataCommand)>,
    ) {
        let session_id = message.session_id();
        match self.sessions.get_mut(&session_id) {
            Some(Session {
                handler, discovery, ..
            }) => {
                let (addresses, responses) = discovery.handle_message(message, handler);
                let maybe_command = match addresses.is_empty() {
                    false => {
                        debug!(target: "aleph-network", "Adding addresses for session {:?} to reserved: {:?}", session_id, addresses);
                        self.connections
                            .add_peers(session_id, addresses.iter().flat_map(get_peer_id));
                        Some(ConnectionCommand::AddReserved(
                            addresses.into_iter().map(|address| address.0).collect(),
                        ))
                    }
                    true => None,
                };
                (
                    maybe_command,
                    responses.into_iter().map(Self::network_message).collect(),
                )
            }
            None => {
                debug!(target: "aleph-network", "Received message from unknown session: {:?}", message);
                (None, Vec::new())
            }
        }
    }

    /// Sends the data to the identified session.
    pub fn send_session_data(&self, session_id: &SessionId, data: D) -> Result<(), Error> {
        match self
            .sessions
            .get(session_id)
            .map(|session| session.data_for_user.as_ref())
            .flatten()
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
    ) -> Result<
        (
            Option<ConnectionCommand>,
            Vec<(NetworkData<D>, DataCommand)>,
        ),
        SessionHandlerError,
    > {
        let (pre_session, result_for_user) = match self.to_retry.pop() {
            Some(to_retry) => to_retry,
            None => return Ok((None, Vec::new())),
        };
        match pre_session {
            PreSession::Validator(pre_session) => {
                self.handle_validator_presession(pre_session, result_for_user)
                    .await
            }
            PreSession::Nonvalidator(pre_session) => {
                self.handle_nonvalidator_presession(pre_session).await?;
                Ok((None, Vec::new()))
            }
        }
    }
}

/// Input/output interface for the connectiona manager service.
pub struct IO<D: Data> {
    commands_for_network: mpsc::UnboundedSender<ConnectionCommand>,
    messages_for_network: mpsc::UnboundedSender<(NetworkData<D>, DataCommand)>,
    commands_from_user: mpsc::UnboundedReceiver<SessionCommand<D>>,
    messages_from_user: mpsc::UnboundedReceiver<(D, SessionId, Recipient)>,
    messages_from_network: mpsc::UnboundedReceiver<NetworkData<D>>,
}

/// Errors that can happen during the network service operations.
#[derive(Debug, PartialEq)]
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

impl<D: Data> IO<D> {
    pub fn new(
        commands_for_network: mpsc::UnboundedSender<ConnectionCommand>,
        messages_for_network: mpsc::UnboundedSender<(NetworkData<D>, DataCommand)>,
        commands_from_user: mpsc::UnboundedReceiver<SessionCommand<D>>,
        messages_from_user: mpsc::UnboundedReceiver<(D, SessionId, Recipient)>,
        messages_from_network: mpsc::UnboundedReceiver<NetworkData<D>>,
    ) -> IO<D> {
        IO {
            commands_for_network,
            messages_for_network,
            commands_from_user,
            messages_from_user,
            messages_from_network,
        }
    }

    fn send_data(&self, to_send: (NetworkData<D>, DataCommand)) -> Result<(), Error> {
        self.messages_for_network
            .unbounded_send(to_send)
            .map_err(|_| Error::NetworkSend)
    }

    fn send_command(&self, to_send: ConnectionCommand) -> Result<(), Error> {
        self.commands_for_network
            .unbounded_send(to_send)
            .map_err(|_| Error::CommandSend)
    }

    fn send(
        &self,
        (maybe_command, data): (
            Option<ConnectionCommand>,
            Vec<(NetworkData<D>, DataCommand)>,
        ),
    ) -> Result<(), Error> {
        if let Some(command) = maybe_command {
            self.send_command(command)?;
        }
        for data_to_send in data {
            self.send_data(data_to_send)?;
        }
        Ok(())
    }

    fn on_network_message<NI: NetworkIdentity>(
        &self,
        service: &mut Service<NI, D>,
        message: NetworkData<D>,
    ) -> Result<(), Error> {
        use NetworkData::*;
        match message {
            Meta(message) => self.send(service.on_discovery_message(message)),
            Data(data, session_id) => service.send_session_data(&session_id, data),
        }
    }

    /// Run the connection manager service with this IO.
    pub async fn run<NI: NetworkIdentity>(
        mut self,
        mut service: Service<NI, D>,
    ) -> Result<(), Error> {
        let mut maintenance = interval(service.maintenance_period);
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
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Config, Error, Service, SessionCommand};
    use crate::{
        network::{
            manager::{
                testing::{crypto_basics, MockNetworkIdentity},
                DiscoveryMessage, NetworkData,
            },
            ConnectionCommand, DataCommand, Protocol,
        },
        SessionId,
    };
    use aleph_bft::Recipient;
    use futures::{channel::oneshot, StreamExt};
    use std::time::Duration;

    const NUM_NODES: usize = 7;
    const MAINTENANCE_PERIOD: Duration = Duration::from_secs(120);
    const DISCOVERY_PERIOD: Duration = Duration::from_secs(60);

    fn build() -> Service<MockNetworkIdentity, i32> {
        Service::new(
            MockNetworkIdentity::new(),
            Config::new(MAINTENANCE_PERIOD, DISCOVERY_PERIOD),
        )
    }

    #[tokio::test]
    async fn starts_nonvalidator_session() {
        let mut service = build();
        let (_, verifier) = crypto_basics(NUM_NODES).await;
        let session_id = SessionId(43);
        let (maybe_command, data_commands) = service
            .on_command(SessionCommand::StartNonvalidator(session_id, verifier))
            .await
            .unwrap();
        assert!(maybe_command.is_none());
        assert!(data_commands.is_empty());
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
        let (maybe_command, data_commands) = service
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
        assert_eq!(data_commands.len(), 1);
        assert!(data_commands
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
        let (maybe_command, data_commands) = service
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
        assert_eq!(data_commands.len(), 1);
        assert!(data_commands
            .iter()
            .all(|(_, command)| command == &DataCommand::Broadcast));
        assert_eq!(service.send_session_data(&session_id, -43), Ok(()));
        let mut data_from_network = result_from_service.await.unwrap();
        assert_eq!(data_from_network.next().await, Some(-43));
        let (maybe_command, data_commands) = service
            .on_command(SessionCommand::Stop(session_id))
            .await
            .unwrap();
        assert!(maybe_command.is_none());
        assert!(data_commands.is_empty());
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
        let (_, data_commands) = other_service
            .on_command(SessionCommand::StartValidator(
                session_id, verifier, node_id, pen, None,
            ))
            .await
            .unwrap();
        let broadcast = match data_commands[0].clone() {
            (NetworkData::Meta(broadcast), DataCommand::Broadcast) => broadcast,
            _ => panic!(
                "Expected discovery massage broadcast, got: {:?}",
                data_commands[0]
            ),
        };
        let addresses = match &broadcast {
            DiscoveryMessage::AuthenticationBroadcast((auth_data, _)) => auth_data.addresses(),
            _ => panic!("Expected an authentication broadcast, got {:?}", broadcast),
        };
        let (maybe_command, data_commands) = service.on_discovery_message(broadcast);
        assert_eq!(
            maybe_command,
            Some(ConnectionCommand::AddReserved(
                addresses.into_iter().map(|address| address.0).collect()
            ))
        );
        assert_eq!(data_commands.len(), 2);
        assert!(data_commands
            .iter()
            .any(|(_, command)| command == &DataCommand::Broadcast));
        assert!(data_commands
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
        let (_, data_commands) = other_service
            .on_command(SessionCommand::StartValidator(
                session_id, verifier, node_id, pen, None,
            ))
            .await
            .unwrap();
        let broadcast = match data_commands[0].clone() {
            (NetworkData::Meta(broadcast), DataCommand::Broadcast) => broadcast,
            _ => panic!(
                "Expected discovery massage broadcast, got: {:?}",
                data_commands[0]
            ),
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
