use futures::channel::{mpsc, oneshot};

use crate::{
    abft::Recipient,
    crypto::{AuthorityPen, AuthorityVerifier},
    network::{
        data::{
            component::{Sender, SimpleNetwork},
            SendError,
        },
        Data, SessionCommand,
    },
    NodeIndex, SessionId,
};

/// Sends data within a single session.
#[derive(Clone)]
pub struct SessionSender<D: Data> {
    session_id: SessionId,
    messages_for_network: mpsc::UnboundedSender<(D, SessionId, Recipient)>,
}

impl<D: Data> Sender<D> for SessionSender<D> {
    fn send(&self, data: D, recipient: Recipient) -> Result<(), SendError> {
        self.messages_for_network
            .unbounded_send((data, self.session_id, recipient))
            .map_err(|_| SendError::SendFailed)
    }
}

/// Sends and receives data within a single session.
type Network<D> = SimpleNetwork<D, mpsc::UnboundedReceiver<D>, SessionSender<D>>;

/// Manages sessions for which the network should be active.
pub struct Manager<D: Data> {
    commands_for_service: mpsc::UnboundedSender<SessionCommand<D>>,
    messages_for_service: mpsc::UnboundedSender<(D, SessionId, Recipient)>,
}

/// What went wrong during a session management operation.
#[derive(Debug)]
pub enum ManagerError {
    CommandSendFailed,
    NetworkReceiveFailed,
}

pub struct IO<D: Data> {
    pub commands_for_service: mpsc::UnboundedSender<SessionCommand<D>>,
    pub messages_for_service: mpsc::UnboundedSender<(D, SessionId, Recipient)>,
}

impl<D: Data> IO<D> {
    pub fn new(
        commands_for_service: mpsc::UnboundedSender<SessionCommand<D>>,
        messages_for_service: mpsc::UnboundedSender<(D, SessionId, Recipient)>,
    ) -> Self {
        IO {
            commands_for_service,
            messages_for_service,
        }
    }
}

impl<D: Data> Manager<D> {
    /// Create a new manager with the given channels to the service.
    pub fn new(io: IO<D>) -> Self {
        Manager {
            commands_for_service: io.commands_for_service,
            messages_for_service: io.messages_for_service,
        }
    }

    /// Start participating or update the verifier in the given session where you are not a
    /// validator.
    pub fn start_nonvalidator_session(
        &self,
        session_id: SessionId,
        verifier: AuthorityVerifier,
    ) -> Result<(), ManagerError> {
        self.commands_for_service
            .unbounded_send(SessionCommand::StartNonvalidator(session_id, verifier))
            .map_err(|_| ManagerError::CommandSendFailed)
    }

    /// Start participating or update the information about the given session where you are a
    /// validator. Returns a session network to be used for sending and receiving data within the
    /// session.
    pub async fn start_validator_session(
        &self,
        session_id: SessionId,
        verifier: AuthorityVerifier,
        node_id: NodeIndex,
        pen: AuthorityPen,
    ) -> Result<Network<D>, ManagerError> {
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

    /// Start participating or update the information about the given session where you are a
    /// validator. Used for early starts when you don't yet need the returned network, but would
    /// like to start discovery.
    pub fn early_start_validator_session(
        &self,
        session_id: SessionId,
        verifier: AuthorityVerifier,
        node_id: NodeIndex,
        pen: AuthorityPen,
    ) -> Result<(), ManagerError> {
        self.commands_for_service
            .unbounded_send(SessionCommand::StartValidator(
                session_id, verifier, node_id, pen, None,
            ))
            .map_err(|_| ManagerError::CommandSendFailed)
    }

    /// Stop participating in the given session.
    pub fn stop_session(&self, session_id: SessionId) -> Result<(), ManagerError> {
        self.commands_for_service
            .unbounded_send(SessionCommand::Stop(session_id))
            .map_err(|_| ManagerError::CommandSendFailed)
    }
}
