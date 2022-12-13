use std::{
    cmp,
    collections::{HashMap, HashSet},
    fmt::Debug,
    time::Duration,
};

use futures::{
    channel::{mpsc, oneshot},
    StreamExt,
};
use log::{debug, info, trace, warn};
use tokio::time::{self, Instant};

use crate::{
    abft::Recipient,
    crypto::{AuthorityPen, AuthorityVerifier},
    network::{
        manager::{
            compatibility::PeerAuthentications, Connections, DataInSession, Discovery,
            DiscoveryMessage, SessionHandler, SessionHandlerError, VersionedAuthentication,
        },
        AddressedData, AddressingInformation, ConnectionCommand, Data, NetworkIdentity, PeerId,
    },
    validator_network::{Network as ValidatorNetwork, PublicKey},
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

struct Session<D: Data, M: Data, A: AddressingInformation + TryFrom<Vec<M>> + Into<Vec<M>>> {
    handler: SessionHandler<M, A>,
    discovery: Discovery<M, A>,
    data_for_user: Option<mpsc::UnboundedSender<D>>,
}

#[derive(Clone)]
/// Stores all data needed for starting validator session
struct PreValidatorSession {
    session_id: SessionId,
    verifier: AuthorityVerifier,
    node_id: NodeIndex,
    pen: AuthorityPen,
}

#[derive(Clone)]
/// Stores all data needed for starting non-validator session
struct PreNonvalidatorSession {
    session_id: SessionId,
    verifier: AuthorityVerifier,
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

/// Actions that the service wants to take as the result of some information. Might contain a
/// command for connecting to or disconnecting from some peers or a message to broadcast for
/// discovery  purposes.
pub struct ServiceActions<M: Data, A: AddressingInformation + TryFrom<Vec<M>> + Into<Vec<M>>> {
    maybe_command: Option<ConnectionCommand<A>>,
    maybe_message: Option<PeerAuthentications<M, A>>,
}

impl<M: Data, A: AddressingInformation + TryFrom<Vec<M>> + Into<Vec<M>>> ServiceActions<M, A> {
    fn noop() -> Self {
        ServiceActions {
            maybe_command: None,
            maybe_message: None,
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
pub struct Service<NI: NetworkIdentity, M: Data, D: Data>
where
    NI::AddressingInformation: TryFrom<Vec<M>> + Into<Vec<M>>,
{
    network_identity: NI,
    connections: Connections<NI::PeerId>,
    sessions: HashMap<SessionId, Session<D, M, NI::AddressingInformation>>,
    discovery_cooldown: Duration,
    maintenance_period: Duration,
    initial_delay: Duration,
}

impl<NI: NetworkIdentity, M: Data + Debug, D: Data> Service<NI, M, D>
where
    NI::AddressingInformation: TryFrom<Vec<M>> + Into<Vec<M>>,
{
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
            discovery_cooldown,
            maintenance_period,
            initial_delay,
        }
    }

    fn delete_reserved(
        to_remove: HashSet<NI::PeerId>,
    ) -> Option<ConnectionCommand<NI::AddressingInformation>> {
        match to_remove.is_empty() {
            true => None,
            false => Some(ConnectionCommand::DelReserved(to_remove)),
        }
    }

    fn finish_session(
        &mut self,
        session_id: SessionId,
    ) -> Option<ConnectionCommand<NI::AddressingInformation>> {
        self.sessions.remove(&session_id);
        Self::delete_reserved(self.connections.remove_session(session_id))
    }

    fn discover_authorities(
        &mut self,
        session_id: &SessionId,
    ) -> Option<PeerAuthentications<M, NI::AddressingInformation>> {
        self.sessions.get_mut(session_id).and_then(
            |Session {
                 handler, discovery, ..
             }| { discovery.discover_authorities(handler) },
        )
    }

    /// Returns all the network messages that should be sent as part of discovery at this moment.
    pub fn discovery(&mut self) -> Vec<PeerAuthentications<M, NI::AddressingInformation>> {
        let sessions: Vec<_> = self.sessions.keys().cloned().collect();
        sessions
            .iter()
            .flat_map(|session_id| self.discover_authorities(session_id))
            .collect()
    }

    async fn start_validator_session(
        &mut self,
        pre_session: PreValidatorSession,
        address: NI::AddressingInformation,
    ) -> (
        Option<PeerAuthentications<M, NI::AddressingInformation>>,
        mpsc::UnboundedReceiver<D>,
    ) {
        let PreValidatorSession {
            session_id,
            verifier,
            node_id,
            pen,
        } = pre_session;
        let handler =
            SessionHandler::new(Some((node_id, pen)), verifier, session_id, address).await;
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
        (self.discover_authorities(&session_id), data_from_network)
    }

    async fn update_validator_session(
        &mut self,
        pre_session: PreValidatorSession,
    ) -> Result<
        (
            ServiceActions<M, NI::AddressingInformation>,
            mpsc::UnboundedReceiver<D>,
        ),
        SessionHandlerError,
    > {
        let address = self.network_identity.identity();
        let session = match self.sessions.get_mut(&pre_session.session_id) {
            Some(session) => session,
            None => {
                let (maybe_message, data_from_network) =
                    self.start_validator_session(pre_session, address).await;
                return Ok((
                    ServiceActions {
                        maybe_command: None,
                        maybe_message,
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
            .update(Some((node_id, pen)), verifier, address)
            .await?
            .iter()
            .map(|address| address.peer_id())
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
                maybe_message: self.discover_authorities(&session_id),
            },
            data_from_network,
        ))
    }

    async fn handle_validator_presession(
        &mut self,
        pre_session: PreValidatorSession,
        result_for_user: Option<oneshot::Sender<mpsc::UnboundedReceiver<D>>>,
    ) -> Result<ServiceActions<M, NI::AddressingInformation>, SessionHandlerError> {
        self.update_validator_session(pre_session)
            .await
            .map(|(actions, data_from_network)| {
                if let Some(result_for_user) = result_for_user {
                    if result_for_user.send(data_from_network).is_err() {
                        warn!(target: "aleph-network", "Failed to send started session.")
                    }
                }
                actions
            })
    }

    async fn start_nonvalidator_session(
        &mut self,
        pre_session: PreNonvalidatorSession,
        address: NI::AddressingInformation,
    ) {
        let PreNonvalidatorSession {
            session_id,
            verifier,
        } = pre_session;
        let handler = SessionHandler::new(None, verifier, session_id, address).await;
        let discovery = Discovery::new(self.discovery_cooldown);
        self.sessions.insert(
            session_id,
            Session {
                handler,
                discovery,
                data_for_user: None,
            },
        );
    }

    async fn update_nonvalidator_session(
        &mut self,
        pre_session: PreNonvalidatorSession,
    ) -> Result<(), SessionHandlerError> {
        let address = self.network_identity.identity();
        let session = match self.sessions.get_mut(&pre_session.session_id) {
            Some(session) => session,
            None => {
                self.start_nonvalidator_session(pre_session, address).await;
                return Ok(());
            }
        };
        session
            .handler
            .update(None, pre_session.verifier, address)
            .await?;
        Ok(())
    }

    async fn handle_nonvalidator_presession(
        &mut self,
        pre_session: PreNonvalidatorSession,
    ) -> Result<(), SessionHandlerError> {
        self.update_nonvalidator_session(pre_session).await
    }

    /// Handle a session command.
    /// Returns actions the service wants to take or an error if the session command is invalid.
    pub async fn on_command(
        &mut self,
        command: SessionCommand<D>,
    ) -> Result<ServiceActions<M, NI::AddressingInformation>, SessionHandlerError> {
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
                maybe_message: None,
            }),
        }
    }

    /// Handle a user request for sending data.
    /// Returns a list of data to be sent over the network.
    pub fn on_user_message(
        &self,
        data: D,
        session_id: SessionId,
        recipient: Recipient,
    ) -> Vec<AddressedData<DataInSession<D>, NI::PeerId>> {
        if let Some(handler) = self
            .sessions
            .get(&session_id)
            .map(|session| &session.handler)
        {
            let to_send = DataInSession { data, session_id };
            match recipient {
                Recipient::Everyone => (0..handler.node_count().0)
                    .map(NodeIndex)
                    .flat_map(|node_id| handler.peer_id(&node_id))
                    .map(|peer_id| (to_send.clone(), peer_id))
                    .collect(),
                Recipient::Node(node_id) => handler
                    .peer_id(&node_id)
                    .into_iter()
                    .map(|peer_id| (to_send.clone(), peer_id))
                    .collect(),
            }
        } else {
            Vec::new()
        }
    }

    /// Handle a discovery message.
    /// Returns actions the service wants to take.
    pub fn on_discovery_message(
        &mut self,
        message: DiscoveryMessage<M, NI::AddressingInformation>,
    ) -> ServiceActions<M, NI::AddressingInformation> {
        use DiscoveryMessage::*;
        let session_id = message.session_id();
        match self.sessions.get_mut(&session_id) {
            Some(Session {
                handler, discovery, ..
            }) => {
                let (maybe_address, maybe_message) = match message {
                    Authentication(authentication) => {
                        discovery.handle_authentication(authentication, handler)
                    }
                    LegacyAuthentication(legacy_authentication) => {
                        discovery.handle_legacy_authentication(legacy_authentication, handler)
                    }
                };
                let maybe_command = match (maybe_address, handler.is_validator()) {
                    (Some(address), true) => {
                        debug!(target: "aleph-network", "Adding addresses for session {:?} to reserved: {:?}", session_id, address);
                        self.connections.add_peers(session_id, [address.peer_id()]);
                        Some(ConnectionCommand::AddReserved([address].into()))
                    }
                    _ => None,
                };
                ServiceActions {
                    maybe_command,
                    maybe_message,
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
                        .map(|(node_id, peer_id)| {
                            format!("{:?}: {}", node_id, peer_id.to_short_string())
                        })
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

/// Input/output interface for the connection manager service.
pub struct IO<
    D: Data,
    M: Data,
    A: AddressingInformation + TryFrom<Vec<M>> + Into<Vec<M>>,
    VN: ValidatorNetwork<A::PeerId, A, DataInSession<D>>,
> where
    A::PeerId: PublicKey,
{
    authentications_for_network: mpsc::UnboundedSender<VersionedAuthentication<M, A>>,
    commands_from_user: mpsc::UnboundedReceiver<SessionCommand<D>>,
    messages_from_user: mpsc::UnboundedReceiver<(D, SessionId, Recipient)>,
    authentications_from_network: mpsc::UnboundedReceiver<VersionedAuthentication<M, A>>,
    validator_network: VN,
}

/// Errors that can happen during the network service operations.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    NetworkSend,
    /// Should never be fatal.
    UserSend,
    /// Should never be fatal.
    NoSession,
    CommandsChannel,
    MessageChannel,
    NetworkChannel,
}

impl<
        D: Data,
        M: Data + Debug,
        A: AddressingInformation + TryFrom<Vec<M>> + Into<Vec<M>>,
        VN: ValidatorNetwork<A::PeerId, A, DataInSession<D>>,
    > IO<D, M, A, VN>
where
    A::PeerId: PublicKey,
{
    pub fn new(
        authentications_for_network: mpsc::UnboundedSender<VersionedAuthentication<M, A>>,
        commands_from_user: mpsc::UnboundedReceiver<SessionCommand<D>>,
        messages_from_user: mpsc::UnboundedReceiver<(D, SessionId, Recipient)>,
        authentications_from_network: mpsc::UnboundedReceiver<VersionedAuthentication<M, A>>,
        validator_network: VN,
    ) -> IO<D, M, A, VN> {
        IO {
            authentications_for_network,
            commands_from_user,
            messages_from_user,
            authentications_from_network,
            validator_network,
        }
    }

    fn send_data(&self, to_send: AddressedData<DataInSession<D>, A::PeerId>) {
        self.validator_network.send(to_send.0, to_send.1)
    }

    fn send_authentications(
        &self,
        to_send: Vec<VersionedAuthentication<M, A>>,
    ) -> Result<(), Error> {
        for auth in to_send {
            self.authentications_for_network
                .unbounded_send(auth)
                .map_err(|_| Error::NetworkSend)?;
        }
        Ok(())
    }

    fn handle_connection_command(&mut self, connection_command: ConnectionCommand<A>) {
        match connection_command {
            ConnectionCommand::AddReserved(addresses) => {
                for address in addresses {
                    self.validator_network
                        .add_connection(address.peer_id(), address);
                }
            }
            ConnectionCommand::DelReserved(peers) => {
                for peer in peers {
                    self.validator_network.remove_connection(peer);
                }
            }
        };
    }

    fn handle_service_actions(
        &mut self,
        ServiceActions {
            maybe_command,
            maybe_message,
        }: ServiceActions<M, A>,
    ) -> Result<(), Error> {
        if let Some(command) = maybe_command {
            self.handle_connection_command(command);
        }
        if let Some(message) = maybe_message {
            self.send_authentications(message.into())?;
        }
        Ok(())
    }

    /// Run the connection manager service with this IO.
    pub async fn run<NI: NetworkIdentity<AddressingInformation = A, PeerId = A::PeerId>>(
        mut self,
        mut service: Service<NI, M, D>,
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
                            Ok(to_send) => self.handle_service_actions(to_send)?,
                            Err(e) => warn!(target: "aleph-network", "Failed to update handler: {:?}", e),
                        },
                        None => return Err(Error::CommandsChannel),
                    }
                },
                maybe_message = self.messages_from_user.next() => {
                    trace!(target: "aleph-network", "Manager received a message from user");
                    match maybe_message {
                        Some((message, session_id, recipient)) => for message in service.on_user_message(message, session_id, recipient) {
                            self.send_data(message);
                        },
                        None => return Err(Error::MessageChannel),
                    }
                },
                maybe_data = self.validator_network.next() => {
                    trace!(target: "aleph-network", "Manager received some data from network");
                    match maybe_data {
                        Some(DataInSession{data, session_id}) => if let Err(e) = service.send_session_data(&session_id, data) {
                            match e {
                                Error::UserSend => trace!(target: "aleph-network", "Failed to send to user in session."),
                                Error::NoSession => trace!(target: "aleph-network", "Received message for unknown session."),
                                _ => return Err(e),
                            }
                        },
                        None => return Err(Error::NetworkChannel),
                    }
                },
                maybe_authentication = self.authentications_from_network.next() => {
                    trace!(target: "aleph-network", "Manager received an authentication from network");
                    match maybe_authentication {
                        Some(authentication) => match authentication.try_into() {
                            Ok(message) => self.handle_service_actions(service.on_discovery_message(message))?,
                            Err(e) => warn!(target: "aleph-network", "Error casting versioned authentication to discovery message: {:?}", e),
                        },
                        None => return Err(Error::NetworkChannel),
                    }
                },
                _ = maintenance.tick() => {
                    debug!(target: "aleph-network", "Manager starts maintenence");
                    for to_send in service.discovery() {
                        self.send_authentications(to_send.into())?;
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
    use std::{iter, time::Duration};

    use futures::{channel::oneshot, StreamExt};

    use super::{Config, Error, Service, ServiceActions, SessionCommand};
    use crate::{
        network::{
            manager::{compatibility::PeerAuthentications, DataInSession, DiscoveryMessage},
            mock::crypto_basics,
            ConnectionCommand,
        },
        testing::mocks::validator_network::{random_address, MockAddressingInformation},
        Recipient, SessionId,
    };

    const NUM_NODES: usize = 7;
    const MAINTENANCE_PERIOD: Duration = Duration::from_secs(120);
    const DISCOVERY_PERIOD: Duration = Duration::from_secs(60);
    const INITIAL_DELAY: Duration = Duration::from_secs(5);

    fn build() -> Service<MockAddressingInformation, MockAddressingInformation, i32> {
        Service::new(
            random_address(),
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
            maybe_message,
        } = service
            .on_command(SessionCommand::StartNonvalidator(session_id, verifier))
            .await
            .unwrap();
        assert!(maybe_command.is_none());
        assert!(maybe_message.is_none());
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
            maybe_message,
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
        assert!(maybe_message.is_some());
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
            maybe_message,
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
        assert!(maybe_message.is_some());
        assert_eq!(service.send_session_data(&session_id, -43), Ok(()));
        let mut data_from_network = result_from_service.await.unwrap();
        assert_eq!(data_from_network.next().await, Some(-43));
        let ServiceActions {
            maybe_command,
            maybe_message,
        } = service
            .on_command(SessionCommand::Stop(session_id))
            .await
            .unwrap();
        assert!(maybe_command.is_none());
        assert!(maybe_message.is_none());
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
        let ServiceActions { maybe_message, .. } = other_service
            .on_command(SessionCommand::StartValidator(
                session_id, verifier, node_id, pen, None,
            ))
            .await
            .unwrap();
        let message = maybe_message.expect("there should be a discovery message");
        let (address, message) = match message {
            PeerAuthentications::Both(authentication, _) => (
                authentication.0.address(),
                DiscoveryMessage::Authentication(authentication),
            ),
            message => panic!("Expected both authentications, got {:?}", message),
        };
        let ServiceActions {
            maybe_command,
            maybe_message,
        } = service.on_discovery_message(message);
        assert_eq!(
            maybe_command,
            Some(ConnectionCommand::AddReserved(
                iter::once(address).collect()
            ))
        );
        assert!(maybe_message.is_some());
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
        let ServiceActions { maybe_message, .. } = other_service
            .on_command(SessionCommand::StartValidator(
                session_id, verifier, node_id, pen, None,
            ))
            .await
            .unwrap();
        let message = match maybe_message.expect("there should be a discovery message") {
            PeerAuthentications::Both(authentication, _) => {
                DiscoveryMessage::Authentication(authentication)
            }
            message => panic!("Expected both authentications, got {:?}", message),
        };
        service.on_discovery_message(message);
        let messages = service.on_user_message(2137, session_id, Recipient::Everyone);
        assert_eq!(messages.len(), 1);
        let (network_data, _) = &messages[0];
        assert_eq!(
            network_data,
            &DataInSession {
                data: 2137,
                session_id
            }
        );
    }
}
