use crate::{
    crypto::{AuthorityPen, AuthorityVerifier},
    new_network::{ComponentNetwork, SendError, SenderComponent, SessionCommand},
    NodeIndex, SessionId,
};
use aleph_bft::Recipient;
use codec::Codec;
use futures::channel::mpsc;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Sends data within a single session.
#[derive(Clone)]
pub struct Sender<D: Clone + Codec + Send> {
    session_id: SessionId,
    messages_for_network: mpsc::UnboundedSender<(D, SessionId, Recipient)>,
}

impl<D: Clone + Codec + Send> SenderComponent<D> for Sender<D> {
    fn send(&self, data: D, recipient: Recipient) -> Result<(), SendError> {
        self.messages_for_network
            .unbounded_send((data, self.session_id, recipient))
            .map_err(|_| SendError::SendFailed)
    }
}

/// Sends and receives data within a single session.
pub struct Network<D: Clone + Codec + Send> {
    sender: Sender<D>,
    receiver: Arc<Mutex<mpsc::UnboundedReceiver<D>>>,
}

impl<D: Clone + Codec + Send> ComponentNetwork<D> for Network<D> {
    type S = Sender<D>;
    type R = mpsc::UnboundedReceiver<D>;
    fn sender(&self) -> &Self::S {
        &self.sender
    }
    fn receiver(&self) -> Arc<Mutex<Self::R>> {
        self.receiver.clone()
    }
}

/// Manages sessions for which the network should be active.
pub struct Manager<D: Clone + Codec + Send> {
    commands_for_service: mpsc::UnboundedSender<SessionCommand<D>>,
    messages_for_service: mpsc::UnboundedSender<(D, SessionId, Recipient)>,
}

/// What went wrond during a session management operation.
pub enum ManagerError {
    CommandSendFailed,
}

impl<D: Clone + Codec + Send> Manager<D> {
    /// Create a new manager with the given channels to the service.
    pub fn new(
        commands_for_service: mpsc::UnboundedSender<SessionCommand<D>>,
        messages_for_service: mpsc::UnboundedSender<(D, SessionId, Recipient)>,
    ) -> Self {
        Manager {
            commands_for_service,
            messages_for_service,
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
    pub fn start_validator_session(
        &self,
        session_id: SessionId,
        verifier: AuthorityVerifier,
        node_id: NodeIndex,
        pen: AuthorityPen,
    ) -> Result<Network<D>, ManagerError> {
        let (data_for_user, data_from_network) = mpsc::unbounded();
        self.commands_for_service
            .unbounded_send(SessionCommand::StartValidator(
                session_id,
                verifier,
                node_id,
                pen,
                data_for_user,
            ))
            .map_err(|_| ManagerError::CommandSendFailed)?;
        let messages_for_network = self.messages_for_service.clone();
        Ok(Network {
            sender: Sender {
                session_id,
                messages_for_network,
            },
            receiver: Arc::new(Mutex::new(data_from_network)),
        })
    }

    /// Stop participating in the given session.
    pub fn stop_session(&self, session_id: SessionId) -> Result<(), ManagerError> {
        self.commands_for_service
            .unbounded_send(SessionCommand::Stop(session_id))
            .map_err(|_| ManagerError::CommandSendFailed)
    }
}
