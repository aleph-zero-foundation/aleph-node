use std::{
    cmp,
    fmt::{Debug, Display, Error as FmtError, Formatter},
    time::Duration,
};

use futures::{
    channel::{mpsc, oneshot},
    StreamExt,
};
use log::{debug, trace, warn};
use network_clique::{Network as CliqueNetwork, PublicKey};
use tokio::time::{self, Instant};

use crate::{
    abft::Recipient,
    crypto::{AuthorityPen, AuthorityVerifier},
    network::{
        session::{
            data::DataInSession,
            manager::{
                AddressedData, ConnectionCommand, Manager, ManagerActions, PreNonvalidatorSession,
                PreValidatorSession, SendError,
            },
            Network, SessionHandlerError, SessionManager, SessionSender, VersionedAuthentication,
        },
        AddressingInformation, Data, GossipNetwork, NetworkIdentity,
    },
    MillisecsPerBlock, NodeIndex, SessionId, SessionPeriod, STATUS_REPORT_INTERVAL,
};

/// Commands for manipulating sessions, stopping them and starting both validator and non-validator
/// sessions.
enum SessionCommand<D: Data> {
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

/// Manages sessions for which the network should be active.
struct ManagerInterface<D: Data> {
    commands_for_service: mpsc::UnboundedSender<SessionCommand<D>>,
    messages_for_service: mpsc::UnboundedSender<(D, SessionId, Recipient)>,
}

/// What went wrong during a session management operation.
#[derive(Debug)]
pub enum ManagerError {
    CommandSendFailed,
    NetworkReceiveFailed,
}

impl Display for ManagerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use ManagerError::*;
        match self {
            CommandSendFailed => write!(f, "failed to send a command to the service"),
            NetworkReceiveFailed => write!(f, "the service did not return a network"),
        }
    }
}

#[async_trait::async_trait]
impl<D: Data> SessionManager<D> for ManagerInterface<D> {
    type Error = ManagerError;

    fn start_nonvalidator_session(
        &self,
        session_id: SessionId,
        verifier: AuthorityVerifier,
    ) -> Result<(), Self::Error> {
        self.commands_for_service
            .unbounded_send(SessionCommand::StartNonvalidator(session_id, verifier))
            .map_err(|_| ManagerError::CommandSendFailed)
    }

    async fn start_validator_session(
        &self,
        session_id: SessionId,
        verifier: AuthorityVerifier,
        node_id: NodeIndex,
        pen: AuthorityPen,
    ) -> Result<Network<D>, Self::Error> {
        let (result_for_us, result_from_service) = oneshot::channel();
        self.commands_for_service
            .unbounded_send(SessionCommand::StartValidator(
                session_id,
                verifier,
                node_id,
                pen,
                Some(result_for_us),
            ))
            .map_err(|_| ManagerError::CommandSendFailed)?;

        let data_from_network = result_from_service
            .await
            .map_err(|_| ManagerError::NetworkReceiveFailed)?;
        let messages_for_network = self.messages_for_service.clone();

        Ok(Network::new(
            data_from_network,
            SessionSender {
                session_id,
                messages_for_network,
            },
        ))
    }

    fn early_start_validator_session(
        &self,
        session_id: SessionId,
        verifier: AuthorityVerifier,
        node_id: NodeIndex,
        pen: AuthorityPen,
    ) -> Result<(), Self::Error> {
        self.commands_for_service
            .unbounded_send(SessionCommand::StartValidator(
                session_id, verifier, node_id, pen, None,
            ))
            .map_err(|_| ManagerError::CommandSendFailed)
    }

    fn stop_session(&self, session_id: SessionId) -> Result<(), Self::Error> {
        self.commands_for_service
            .unbounded_send(SessionCommand::Stop(session_id))
            .map_err(|_| ManagerError::CommandSendFailed)
    }
}

/// Configuration for the session manager. Controls how often the maintenance and
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

/// The connection manager service.
pub struct Service<
    D: Data,
    NI: NetworkIdentity,
    CN: CliqueNetwork<NI::PeerId, NI::AddressingInformation, DataInSession<D>>,
    GN: GossipNetwork<VersionedAuthentication<NI::AddressingInformation>>,
> where
    NI::PeerId: PublicKey,
{
    manager: Manager<NI, D>,
    commands_from_user: mpsc::UnboundedReceiver<SessionCommand<D>>,
    messages_from_user: mpsc::UnboundedReceiver<(D, SessionId, Recipient)>,
    validator_network: CN,
    gossip_network: GN,
    maintenance_period: Duration,
    initial_delay: Duration,
}

/// Errors that can happen during the network service operations.
#[derive(Debug, PartialEq, Eq)]
pub enum Error<GE: Display> {
    CommandsChannel,
    MessageChannel,
    ValidatorNetwork,
    GossipNetwork(GE),
}

impl<GE: Display> Display for Error<GE> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use Error::*;
        match self {
            CommandsChannel => write!(f, "commands channel unexpectedly closed"),
            MessageChannel => write!(f, "message channel unexpectedly closed"),
            ValidatorNetwork => write!(f, "validator network unexpectedly done"),
            GossipNetwork(e) => write!(f, "gossip network unexpectedly done: {}", e),
        }
    }
}

impl<
        D: Data,
        NI: NetworkIdentity,
        CN: CliqueNetwork<NI::PeerId, NI::AddressingInformation, DataInSession<D>>,
        GN: GossipNetwork<VersionedAuthentication<NI::AddressingInformation>>,
    > Service<D, NI, CN, GN>
where
    NI::PeerId: PublicKey,
{
    pub fn new(
        network_identity: NI,
        validator_network: CN,
        gossip_network: GN,
        config: Config,
    ) -> (
        Service<D, NI, CN, GN>,
        impl SessionManager<D, Error = ManagerError>,
    ) {
        let Config {
            discovery_cooldown,
            maintenance_period,
            initial_delay,
        } = config;
        let manager = Manager::new(network_identity, discovery_cooldown);
        let (commands_for_service, commands_from_user) = mpsc::unbounded();
        let (messages_for_service, messages_from_user) = mpsc::unbounded();
        (
            Service {
                manager,
                commands_from_user,
                messages_from_user,
                validator_network,
                gossip_network,
                maintenance_period,
                initial_delay,
            },
            ManagerInterface {
                commands_for_service,
                messages_for_service,
            },
        )
    }

    fn send_data(&self, to_send: AddressedData<DataInSession<D>, NI::PeerId>) {
        self.validator_network.send(to_send.0, to_send.1)
    }

    fn send_authentications(
        &mut self,
        to_send: Vec<VersionedAuthentication<NI::AddressingInformation>>,
    ) -> Result<(), Error<GN::Error>> {
        for auth in to_send {
            self.gossip_network
                .broadcast(auth)
                .map_err(Error::GossipNetwork)?;
        }
        Ok(())
    }

    fn handle_connection_command(
        &mut self,
        connection_command: ConnectionCommand<NI::AddressingInformation>,
    ) {
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

    fn handle_manager_actions(
        &mut self,
        ManagerActions {
            maybe_command,
            maybe_message,
        }: ManagerActions<NI::AddressingInformation>,
    ) -> Result<(), Error<GN::Error>> {
        if let Some(command) = maybe_command {
            self.handle_connection_command(command);
        }
        if let Some(message) = maybe_message {
            self.send_authentications(message.into())?;
        }
        Ok(())
    }

    /// Handle a session command.
    /// Returns actions the manager wants to take or an error if the session command is invalid.
    async fn handle_command(
        &mut self,
        command: SessionCommand<D>,
    ) -> Result<ManagerActions<NI::AddressingInformation>, SessionHandlerError> {
        use SessionCommand::*;
        match command {
            StartValidator(session_id, verifier, node_id, pen, result_for_user) => {
                let pre_session = PreValidatorSession {
                    session_id,
                    verifier,
                    node_id,
                    pen,
                };
                let (actions, data_from_network) =
                    self.manager.update_validator_session(pre_session).await?;
                if let Some(result_for_user) = result_for_user {
                    if result_for_user.send(data_from_network).is_err() {
                        warn!(target: "aleph-network", "Failed to send started session.")
                    }
                }
                Ok(actions)
            }
            StartNonvalidator(session_id, verifier) => {
                let pre_session = PreNonvalidatorSession {
                    session_id,
                    verifier,
                };
                self.manager.update_nonvalidator_session(pre_session).await
            }
            Stop(session_id) => Ok(self.manager.finish_session(session_id)),
        }
    }

    /// Run the connection manager service.
    pub async fn run(mut self) -> Result<(), Error<GN::Error>> {
        // Initial delay is needed so that Network is fully set up and we received some first discovery broadcasts from other nodes.
        // Otherwise this might cause first maintenance never working, as it happens before first broadcasts.
        let mut maintenance =
            time::interval_at(Instant::now() + self.initial_delay, self.maintenance_period);

        let mut status_ticker = time::interval(STATUS_REPORT_INTERVAL);
        loop {
            trace!(target: "aleph-network", "Manager Loop started a next iteration");
            tokio::select! {
                maybe_command = self.commands_from_user.next() => {
                    trace!(target: "aleph-network", "Manager received a command from user");
                    match maybe_command {
                        Some(command) => match self.handle_command(command).await {
                            Ok(to_send) => self.handle_manager_actions(to_send)?,
                            Err(e) => warn!(target: "aleph-network", "Failed to update handler: {:?}", e),
                        },
                        None => return Err(Error::CommandsChannel),
                    }
                },
                maybe_message = self.messages_from_user.next() => {
                    trace!(target: "aleph-network", "Manager received a message from user");
                    match maybe_message {
                        Some((message, session_id, recipient)) => for message in self.manager.on_user_message(message, session_id, recipient) {
                            self.send_data(message);
                        },
                        None => return Err(Error::MessageChannel),
                    }
                },
                maybe_data = self.validator_network.next() => {
                    trace!(target: "aleph-network", "Manager received some data from network");
                    match maybe_data {
                        Some(DataInSession{data, session_id}) => if let Err(e) = self.manager.send_session_data(&session_id, data) {
                            match e {
                                SendError::UserSend => trace!(target: "aleph-network", "Failed to send to user in session."),
                                SendError::NoSession => trace!(target: "aleph-network", "Received message for unknown session."),
                            }
                        },
                        None => return Err(Error::ValidatorNetwork),
                    }
                },
                maybe_authentication = self.gossip_network.next() => {
                    let (authentication, _) = maybe_authentication.map_err(Error::GossipNetwork)?;
                    trace!(target: "aleph-network", "Manager received an authentication from network");
                    match authentication.try_into() {
                        Ok(message) => {
                            let manager_actions = self.manager.on_discovery_message(message);
                            self.handle_manager_actions(manager_actions)?
                        },
                        Err(e) => debug!(target: "aleph-network", "Could not cast versioned authentication in discovery message: {:?}", e),
                    }
                },
                _ = maintenance.tick() => {
                    debug!(target: "aleph-network", "Manager starts maintenence");
                    for to_send in self.manager.discovery() {
                        self.send_authentications(to_send.into())?;
                    }
                },
                _ = status_ticker.tick() => {
                    self.manager.status_report();
                }
            }
        }
    }
}
