use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    time::Duration,
};

use futures::channel::mpsc;
use log::{debug, info};

use crate::{
    abft::Recipient,
    crypto::{AuthorityPen, AuthorityVerifier},
    network::{
        address_cache::{ValidatorAddressCacheUpdater, ValidatorAddressingInfo},
        session::{
            data::DataInSession, Authentication, Connections, Discovery, DiscoveryMessage,
            SessionHandler, SessionHandlerError,
        },
        AddressingInformation, Data, NetworkIdentity, PeerId,
    },
    NodeIndex, SessionId,
};

/// Commands for manipulating the reserved peers set.
#[derive(Debug, PartialEq, Eq)]
pub enum ConnectionCommand<A: AddressingInformation> {
    AddReserved(HashSet<A>),
    DelReserved(HashSet<A::PeerId>),
}

// In practice D: Data and P: PeerId, but we cannot require that in type aliases.
pub type AddressedData<D, P> = (D, P);

struct Session<D: Data, A: AddressingInformation> {
    handler: SessionHandler<A>,
    discovery: Discovery<A>,
    data_for_user: Option<mpsc::UnboundedSender<D>>,
}

#[derive(Clone)]
/// Stores all data needed for starting validator session
pub struct PreValidatorSession {
    pub session_id: SessionId,
    pub verifier: AuthorityVerifier,
    pub node_id: NodeIndex,
    pub pen: AuthorityPen,
}

#[derive(Clone)]
/// Stores all data needed for starting non-validator session
pub struct PreNonvalidatorSession {
    pub session_id: SessionId,
    pub verifier: AuthorityVerifier,
}

/// Actions that the manager wants to take as the result of some information. Might contain a
/// command for connecting to or disconnecting from some peers or a message to broadcast for
/// discovery  purposes.
pub struct ManagerActions<A: AddressingInformation> {
    pub maybe_command: Option<ConnectionCommand<A>>,
    pub maybe_message: Option<Authentication<A>>,
}

impl<A: AddressingInformation> ManagerActions<A> {
    fn noop() -> Self {
        ManagerActions {
            maybe_command: None,
            maybe_message: None,
        }
    }
}

/// The connection manager. It handles the abstraction over the network we build to support
/// separate sessions. This includes:
/// 1. Starting and ending specific sessions on user demand.
/// 2. Forwarding in-session user messages to the network using session handlers for address
///    translation.
/// 3. Handling network messages:
///    1. In-session messages are forwarded to the user.
///    2. Authentication messages forwarded to session handlers.
/// 4. Running periodic maintenance, mostly related to node discovery.
pub struct Manager<NI: NetworkIdentity, D: Data, VCU: ValidatorAddressCacheUpdater> {
    network_identity: NI,
    connections: Connections<NI::PeerId>,
    sessions: HashMap<SessionId, Session<D, NI::AddressingInformation>>,
    validator_address_cache_updater: VCU,
    discovery_cooldown: Duration,
}

/// Error when trying to forward data from the network to the user, should never be fatal.
#[derive(Debug, PartialEq, Eq)]
pub enum SendError {
    UserSend,
    NoSession,
}

impl<NI: NetworkIdentity, D: Data, VCU: ValidatorAddressCacheUpdater> Manager<NI, D, VCU> {
    /// Create a new connection manager.
    pub fn new(
        network_identity: NI,
        validator_address_cache_updater: VCU,
        discovery_cooldown: Duration,
    ) -> Self {
        Manager {
            network_identity,
            connections: Connections::new(),
            sessions: HashMap::new(),
            validator_address_cache_updater,
            discovery_cooldown,
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

    /// Ends a session.
    pub fn finish_session(
        &mut self,
        session_id: SessionId,
    ) -> ManagerActions<NI::AddressingInformation> {
        self.sessions.remove(&session_id);
        ManagerActions {
            maybe_command: Self::delete_reserved(self.connections.remove_session(session_id)),
            maybe_message: None,
        }
    }

    fn discover_authorities(
        &mut self,
        session_id: &SessionId,
    ) -> Option<Authentication<NI::AddressingInformation>> {
        self.sessions.get_mut(session_id).and_then(
            |Session {
                 handler, discovery, ..
             }| { discovery.discover_authorities(handler) },
        )
    }

    /// Returns all the network messages that should be sent as part of discovery at this moment.
    pub fn discovery(&mut self) -> Vec<Authentication<NI::AddressingInformation>> {
        let sessions: Vec<_> = self.sessions.keys().cloned().collect();
        sessions
            .iter()
            .flat_map(|session_id| self.discover_authorities(session_id))
            .collect()
    }

    fn start_validator_session(
        &mut self,
        pre_session: PreValidatorSession,
        address: NI::AddressingInformation,
    ) -> (
        Option<Authentication<NI::AddressingInformation>>,
        mpsc::UnboundedReceiver<D>,
    ) {
        let PreValidatorSession {
            session_id,
            verifier,
            node_id,
            pen,
        } = pre_session;
        let handler = SessionHandler::new(Some((node_id, pen)), verifier, session_id, address);
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

    /// Starts or updates a validator session.
    pub fn update_validator_session(
        &mut self,
        pre_session: PreValidatorSession,
    ) -> Result<
        (
            ManagerActions<NI::AddressingInformation>,
            mpsc::UnboundedReceiver<D>,
        ),
        SessionHandlerError,
    > {
        let address = self.network_identity.identity();
        let session = match self.sessions.get_mut(&pre_session.session_id) {
            Some(session) => session,
            None => {
                let (maybe_message, data_from_network) =
                    self.start_validator_session(pre_session, address);
                return Ok((
                    ManagerActions {
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
        self.validator_address_cache_updater.update(
            node_id,
            ValidatorAddressingInfo {
                session: session_id,
                network_level_address: address.address(),
                validator_network_peer_id: address.peer_id().to_string(),
            },
        );

        let peers_to_stay = session
            .handler
            .update(Some((node_id, pen)), verifier, address)?
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
            ManagerActions {
                maybe_command,
                maybe_message: self.discover_authorities(&session_id),
            },
            data_from_network,
        ))
    }

    fn start_nonvalidator_session(
        &mut self,
        pre_session: PreNonvalidatorSession,
        address: NI::AddressingInformation,
    ) {
        let PreNonvalidatorSession {
            session_id,
            verifier,
        } = pre_session;
        let handler = SessionHandler::new(None, verifier, session_id, address);
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

    /// Starts or updates a nonvalidator session.
    pub fn update_nonvalidator_session(
        &mut self,
        pre_session: PreNonvalidatorSession,
    ) -> Result<ManagerActions<NI::AddressingInformation>, SessionHandlerError> {
        let address = self.network_identity.identity();
        match self.sessions.get_mut(&pre_session.session_id) {
            Some(session) => {
                session
                    .handler
                    .update(None, pre_session.verifier, address)?;
            }
            None => {
                self.start_nonvalidator_session(pre_session, address);
            }
        };
        Ok(ManagerActions::noop())
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
    /// Returns actions the manager wants to take.
    pub fn on_discovery_message(
        &mut self,
        message: DiscoveryMessage<NI::AddressingInformation>,
    ) -> ManagerActions<NI::AddressingInformation> {
        let session_id = message.session_id();
        let creator = message.0.creator();
        match self.sessions.get_mut(&session_id) {
            Some(Session {
                handler, discovery, ..
            }) => {
                let (maybe_address, maybe_message) =
                    discovery.handle_authentication(message, handler);
                let mut maybe_command = None;
                if let Some(address) = maybe_address {
                    self.validator_address_cache_updater.update(
                        creator,
                        ValidatorAddressingInfo {
                            session: session_id,
                            network_level_address: address.address(),
                            validator_network_peer_id: address.peer_id().to_string(),
                        },
                    );
                    if handler.is_validator() {
                        debug!(target: "aleph-network", "Adding addresses for session {:?} to reserved: {:?}", session_id, address);
                        self.connections.add_peers(session_id, [address.peer_id()]);
                        maybe_command = Some(ConnectionCommand::AddReserved([address].into()));
                    }
                }
                ManagerActions {
                    maybe_command,
                    maybe_message,
                }
            }
            None => {
                debug!(target: "aleph-network", "Received message from unknown session: {:?}", message);
                ManagerActions::noop()
            }
        }
    }

    /// Sends the data to the identified session.
    pub fn send_session_data(&self, session_id: &SessionId, data: D) -> Result<(), SendError> {
        match self
            .sessions
            .get(session_id)
            .and_then(|session| session.data_for_user.as_ref())
        {
            Some(data_for_user) => data_for_user
                .unbounded_send(data)
                .map_err(|_| SendError::UserSend),
            None => Err(SendError::NoSession),
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
                "authenticated authorities: {authenticated_status}; "
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
                .map(|(session_id, missing)| format!("{session_id:?}: {missing:?}"))
                .collect::<Vec<_>>()
                .join(", ");
            status.push_str(&format!("missing authorities: {missing_status}; "));
        }

        if !authenticated.is_empty() || !missing.is_empty() {
            info!(target: "aleph-network", "{}", status);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{iter, time::Duration};

    use futures::StreamExt;
    use network_clique::mock::{random_address, MockAddressingInformation};

    use super::{
        ConnectionCommand, Manager, ManagerActions, PreNonvalidatorSession, PreValidatorSession,
        SendError,
    };
    use crate::{
        network::{
            address_cache::{test::noop_updater, ValidatorAddressCacheUpdater},
            mock::crypto_basics,
            session::data::DataInSession,
        },
        Recipient, SessionId,
    };

    const NUM_NODES: usize = 7;
    const DISCOVERY_PERIOD: Duration = Duration::from_secs(60);

    fn build() -> Manager<MockAddressingInformation, i32, impl ValidatorAddressCacheUpdater> {
        Manager::new(random_address(), noop_updater(), DISCOVERY_PERIOD)
    }

    #[test]
    fn starts_nonvalidator_session() {
        let mut manager = build();
        let (_, verifier) = crypto_basics(NUM_NODES);
        let session_id = SessionId(43);
        let ManagerActions {
            maybe_command,
            maybe_message,
        } = manager
            .update_nonvalidator_session(PreNonvalidatorSession {
                session_id,
                verifier,
            })
            .unwrap();
        assert!(maybe_command.is_none());
        assert!(maybe_message.is_none());
        assert_eq!(
            manager.send_session_data(&session_id, -43),
            Err(SendError::NoSession)
        );
    }

    #[test]
    fn starts_validator_session() {
        let mut manager = build();
        let (validator_data, verifier) = crypto_basics(NUM_NODES);
        let (node_id, pen) = validator_data[0].clone();
        let session_id = SessionId(43);
        let (
            ManagerActions {
                maybe_command,
                maybe_message,
            },
            _data_from_network,
        ) = manager
            .update_validator_session(PreValidatorSession {
                session_id,
                verifier,
                node_id,
                pen,
            })
            .unwrap();
        assert!(maybe_command.is_none());
        assert!(maybe_message.is_some());
        assert_eq!(manager.send_session_data(&session_id, -43), Ok(()));
    }

    #[tokio::test]
    async fn stops_session() {
        let mut manager = build();
        let (validator_data, verifier) = crypto_basics(NUM_NODES);
        let (node_id, pen) = validator_data[0].clone();
        let session_id = SessionId(43);
        let (
            ManagerActions {
                maybe_command,
                maybe_message,
            },
            mut data_from_network,
        ) = manager
            .update_validator_session(PreValidatorSession {
                session_id,
                verifier,
                node_id,
                pen,
            })
            .unwrap();
        assert!(maybe_command.is_none());
        assert!(maybe_message.is_some());
        assert_eq!(manager.send_session_data(&session_id, -43), Ok(()));
        assert_eq!(data_from_network.next().await, Some(-43));
        let ManagerActions {
            maybe_command,
            maybe_message,
        } = manager.finish_session(session_id);
        assert!(maybe_command.is_none());
        assert!(maybe_message.is_none());
        assert_eq!(
            manager.send_session_data(&session_id, -43),
            Err(SendError::NoSession)
        );
        assert!(data_from_network.next().await.is_none());
    }

    #[test]
    fn handles_broadcast() {
        let mut manager = build();
        let (validator_data, verifier) = crypto_basics(NUM_NODES);
        let (node_id, pen) = validator_data[0].clone();
        let session_id = SessionId(43);
        manager
            .update_validator_session(PreValidatorSession {
                session_id,
                verifier: verifier.clone(),
                node_id,
                pen,
            })
            .unwrap();
        let mut other_manager = build();
        let (node_id, pen) = validator_data[1].clone();
        let (ManagerActions { maybe_message, .. }, _) = other_manager
            .update_validator_session(PreValidatorSession {
                session_id,
                verifier,
                node_id,
                pen,
            })
            .unwrap();
        let message = maybe_message.expect("there should be a discovery message");
        let (address, message) = (message.0.address(), message);
        let ManagerActions {
            maybe_command,
            maybe_message,
        } = manager.on_discovery_message(message);
        assert_eq!(
            maybe_command,
            Some(ConnectionCommand::AddReserved(
                iter::once(address).collect()
            ))
        );
        assert!(maybe_message.is_some());
    }

    #[test]
    fn sends_user_data() {
        let mut manager = build();
        let (validator_data, verifier) = crypto_basics(NUM_NODES);
        let (node_id, pen) = validator_data[0].clone();
        let session_id = SessionId(43);
        manager
            .update_validator_session(PreValidatorSession {
                session_id,
                verifier: verifier.clone(),
                node_id,
                pen,
            })
            .unwrap();
        let mut other_manager = build();
        let (node_id, pen) = validator_data[1].clone();
        let (ManagerActions { maybe_message, .. }, _) = other_manager
            .update_validator_session(PreValidatorSession {
                session_id,
                verifier,
                node_id,
                pen,
            })
            .unwrap();
        let message = maybe_message.expect("there should be a discovery message");
        manager.on_discovery_message(message);
        let messages = manager.on_user_message(2137, session_id, Recipient::Everyone);
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
