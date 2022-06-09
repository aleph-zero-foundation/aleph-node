use crate::{
    network::{
        manager::{Authentication, Multiaddr, SessionHandler},
        DataCommand, PeerId, Protocol,
    },
    NodeIndex, SessionId,
};
use codec::{Decode, Encode};
use log::{debug, trace, warn};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

/// Messages used for discovery and authentication.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub enum DiscoveryMessage {
    AuthenticationBroadcast(Authentication),
    Authentication(Authentication),
}

impl DiscoveryMessage {
    pub fn session_id(&self) -> SessionId {
        use DiscoveryMessage::*;
        match self {
            AuthenticationBroadcast((auth_data, _)) | Authentication((auth_data, _)) => {
                auth_data.session()
            }
        }
    }
}

/// Handles creating and responding to discovery messages.
pub struct Discovery {
    cooldown: Duration,
    last_broadcast: HashMap<NodeIndex, Instant>,
}

type DiscoveryCommand = (DiscoveryMessage, DataCommand);

fn authentication_broadcast(authentication: Authentication) -> DiscoveryCommand {
    (
        DiscoveryMessage::AuthenticationBroadcast(authentication),
        DataCommand::Broadcast,
    )
}

fn response(authentication: Authentication, peer_id: PeerId) -> DiscoveryCommand {
    (
        DiscoveryMessage::Authentication(authentication),
        DataCommand::SendTo(peer_id, Protocol::Generic),
    )
}

impl Discovery {
    /// Create a new discovery handler with the given response/broadcast cooldown.
    pub fn new(cooldown: Duration) -> Self {
        Discovery {
            cooldown,
            last_broadcast: HashMap::new(),
        }
    }

    /// Returns messages that should be sent as part of authority discovery at this moment.
    pub fn discover_authorities(&mut self, handler: &SessionHandler) -> Vec<DiscoveryCommand> {
        let authentication = match handler.authentication() {
            Some(authentication) => authentication,
            None => return Vec::new(),
        };

        let missing_authorities = handler.missing_nodes();
        let node_count = handler.node_count();
        debug!(target: "aleph-network", "{:?}/{:?} authorities known for session {:?}.", node_count.0-missing_authorities.len(), node_count, handler.session_id());
        vec![authentication_broadcast(authentication)]
    }

    /// Checks the authentication using the handler and returns the addresses we should be
    /// connected to if the authentication is correct.
    fn handle_authentication(
        &mut self,
        authentication: Authentication,
        handler: &mut SessionHandler,
    ) -> Vec<Multiaddr> {
        if !handler.handle_authentication(authentication.clone()) {
            return Vec::new();
        }
        authentication.0.addresses()
    }

    fn should_rebroadcast(&self, node_id: &NodeIndex) -> bool {
        match self.last_broadcast.get(node_id) {
            Some(instant) => Instant::now() > *instant + self.cooldown,
            None => true,
        }
    }

    fn handle_broadcast(
        &mut self,
        authentication: Authentication,
        handler: &mut SessionHandler,
    ) -> (Vec<Multiaddr>, Vec<DiscoveryCommand>) {
        debug!(target: "aleph-network", "Handling broadcast with authentication {:?}.", authentication);
        let addresses = self.handle_authentication(authentication.clone(), handler);
        if addresses.is_empty() {
            return (Vec::new(), Vec::new());
        }
        let node_id = authentication.0.creator();
        let mut messages = Vec::new();
        match handler.peer_id(&node_id) {
            Some(peer_id) => {
                if let Some(handler_authentication) = handler.authentication() {
                    messages.push(response(handler_authentication, peer_id));
                }
            }
            None => {
                warn!(target: "aleph-network", "Id of correctly authenticated peer not present.")
            }
        }
        if self.should_rebroadcast(&node_id) {
            trace!(target: "aleph-network", "Rebroadcasting {:?}.", authentication);
            self.last_broadcast.insert(node_id, Instant::now());
            messages.push(authentication_broadcast(authentication));
        }
        (addresses, messages)
    }

    /// Analyzes the provided message and returns all the new multiaddresses we should
    /// be connected to if we want to stay connected to the committee and any messages
    /// that we should send as a result of it.
    pub fn handle_message(
        &mut self,
        message: DiscoveryMessage,
        handler: &mut SessionHandler,
    ) -> (Vec<Multiaddr>, Vec<DiscoveryCommand>) {
        use DiscoveryMessage::*;
        match message {
            AuthenticationBroadcast(authentication) => {
                self.handle_broadcast(authentication, handler)
            }
            Authentication(authentication) => (
                self.handle_authentication(authentication, handler),
                Vec::new(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Discovery, DiscoveryMessage};
    use crate::{
        network::{
            manager::{testing::crypto_basics, SessionHandler},
            DataCommand, Multiaddr,
        },
        SessionId,
    };
    use codec::Encode;
    use sc_network::{
        multiaddr::Protocol as ScProtocol, Multiaddr as ScMultiaddr, PeerId as ScPeerId,
    };
    use std::{net::Ipv4Addr, thread::sleep, time::Duration};

    const NUM_NODES: u8 = 7;
    const MS_COOLDOWN: u64 = 200;

    fn addresses() -> Vec<Multiaddr> {
        (0..NUM_NODES)
            .map(|id| {
                ScMultiaddr::empty()
                    .with(ScProtocol::Ip4(Ipv4Addr::new(192, 168, 1, id)))
                    .with(ScProtocol::Tcp(30333))
                    .with(ScProtocol::P2p(ScPeerId::random().into()))
            })
            .collect()
    }

    async fn build_number(num_nodes: u8) -> (Discovery, Vec<SessionHandler>, SessionHandler) {
        let crypto_basics = crypto_basics(num_nodes.into()).await;
        let mut handlers = Vec::new();
        for (authority_index_and_pen, address) in crypto_basics.0.into_iter().zip(addresses()) {
            handlers.push(
                SessionHandler::new(
                    Some(authority_index_and_pen),
                    crypto_basics.1.clone(),
                    SessionId(43),
                    vec![address.into()],
                )
                .await
                .unwrap(),
            );
        }
        let non_validator = SessionHandler::new(
            None,
            crypto_basics.1.clone(),
            SessionId(43),
            vec![ScMultiaddr::empty()
                .with(ScProtocol::Ip4(Ipv4Addr::new(
                    192,
                    168,
                    1,
                    (handlers.len() - 1) as u8,
                )))
                .with(ScProtocol::Tcp(30333))
                .with(ScProtocol::P2p(ScPeerId::random().into()))
                .into()],
        )
        .await
        .unwrap();
        (
            Discovery::new(Duration::from_millis(MS_COOLDOWN)),
            handlers,
            non_validator,
        )
    }

    async fn build() -> (Discovery, Vec<SessionHandler>, SessionHandler) {
        build_number(NUM_NODES).await
    }

    #[tokio::test]
    async fn broadcasts_when_clueless() {
        for num_nodes in 2..NUM_NODES {
            let (mut discovery, mut handlers, _) = build_number(num_nodes).await;
            let handler = &mut handlers[0];
            let mut messages = discovery.discover_authorities(handler);
            assert_eq!(messages.len(), 1);
            let message = messages.pop().unwrap();
            assert_eq!(
                message,
                (
                    DiscoveryMessage::AuthenticationBroadcast(handler.authentication().unwrap()),
                    DataCommand::Broadcast
                )
            );
        }
    }

    #[tokio::test]
    async fn non_validator_discover_authorities_returns_empty_vector() {
        let (mut discovery, _, non_validator) = build().await;
        let messages = discovery.discover_authorities(&non_validator);
        assert!(messages.is_empty());
    }

    #[tokio::test]
    async fn rebroadcasts_responds_and_accepts_addresses() {
        let (mut discovery, mut handlers, _) = build().await;
        let authentication = handlers[1].authentication().unwrap();
        let handler = &mut handlers[0];
        let (addresses, commands) = discovery.handle_message(
            DiscoveryMessage::AuthenticationBroadcast(authentication.clone()),
            handler,
        );
        assert_eq!(addresses, authentication.0.addresses());
        assert_eq!(commands.len(), 2);
        assert!(commands.iter().any(|command| matches!(command, (
                DiscoveryMessage::AuthenticationBroadcast(rebroadcast_authentication),
                DataCommand::Broadcast,
            ) if rebroadcast_authentication == &authentication)));
        assert!(commands.iter().any(|command| matches!(command, (
                DiscoveryMessage::Authentication(authentication),
                DataCommand::SendTo(_, _),
            ) if *authentication == handler.authentication().unwrap())));
    }

    #[tokio::test]
    async fn non_validators_rebroadcasts_responds() {
        let (mut discovery, handlers, mut non_validator) = build().await;
        let authentication = handlers[1].authentication().unwrap();
        let (addresses, commands) = discovery.handle_message(
            DiscoveryMessage::AuthenticationBroadcast(authentication.clone()),
            &mut non_validator,
        );
        assert_eq!(addresses, authentication.0.addresses());
        assert_eq!(commands.len(), 1);
        assert!(commands.iter().any(|command| matches!(command, (
                DiscoveryMessage::AuthenticationBroadcast(rebroadcast_authentication),
                DataCommand::Broadcast,
            ) if rebroadcast_authentication == &authentication)));
    }

    #[tokio::test]
    async fn does_not_rebroadcast_nor_respond_to_wrong_authentications() {
        let (mut discovery, mut handlers, _) = build().await;
        let (auth_data, _) = handlers[1].authentication().unwrap();
        let (_, signature) = handlers[2].authentication().unwrap();
        let authentication = (auth_data, signature);
        let handler = &mut handlers[0];
        let (addresses, commands) = discovery.handle_message(
            DiscoveryMessage::AuthenticationBroadcast(authentication),
            handler,
        );
        assert!(addresses.is_empty());
        assert!(commands.is_empty());
    }

    #[tokio::test]
    async fn does_not_rebroadcast_quickly_but_still_responds() {
        let (mut discovery, mut handlers, _) = build().await;
        let authentication = handlers[1].authentication().unwrap();
        let handler = &mut handlers[0];
        discovery.handle_message(
            DiscoveryMessage::AuthenticationBroadcast(authentication.clone()),
            handler,
        );
        let (addresses, commands) = discovery.handle_message(
            DiscoveryMessage::AuthenticationBroadcast(authentication.clone()),
            handler,
        );
        assert_eq!(addresses.len(), authentication.0.addresses().len());
        assert_eq!(
            addresses[0].encode(),
            authentication.0.addresses()[0].encode()
        );
        assert_eq!(commands.len(), 1);
        assert!(matches!(&commands[0], (
                DiscoveryMessage::Authentication(authentication),
                DataCommand::SendTo(_, _),
            ) if *authentication == handler.authentication().unwrap()));
    }

    #[tokio::test]
    async fn rebroadcasts_after_cooldown() {
        let (mut discovery, mut handlers, _) = build().await;
        let authentication = handlers[1].authentication().unwrap();
        let handler = &mut handlers[0];
        discovery.handle_message(
            DiscoveryMessage::AuthenticationBroadcast(authentication.clone()),
            handler,
        );
        sleep(Duration::from_millis(MS_COOLDOWN + 5));
        let (addresses, commands) = discovery.handle_message(
            DiscoveryMessage::AuthenticationBroadcast(authentication.clone()),
            handler,
        );
        assert_eq!(addresses, authentication.0.addresses());
        assert!(commands.iter().any(|command| matches!(command, (
                DiscoveryMessage::AuthenticationBroadcast(rebroadcast_authentication),
                DataCommand::Broadcast,
            ) if rebroadcast_authentication == &authentication)));
    }

    #[tokio::test]
    async fn accepts_correct_authentications() {
        let (mut discovery, mut handlers, _) = build().await;
        let expected_address = handlers[1].authentication().unwrap().0.addresses()[0].encode();
        let authentication = handlers[1].authentication().unwrap();
        let handler = &mut handlers[0];
        let (addresses, commands) =
            discovery.handle_message(DiscoveryMessage::Authentication(authentication), handler);
        assert_eq!(addresses.len(), 1);
        let address = addresses[0].encode();
        assert_eq!(address, expected_address);
        assert!(commands.is_empty());
    }

    #[tokio::test]
    async fn does_not_accept_incorrect_authentications() {
        let (mut discovery, mut handlers, _) = build().await;
        let (auth_data, _) = handlers[1].authentication().unwrap();
        let (_, signature) = handlers[2].authentication().unwrap();
        let incorrect_authentication = (auth_data, signature);
        let handler = &mut handlers[0];
        let (addresses, commands) = discovery.handle_message(
            DiscoveryMessage::Authentication(incorrect_authentication),
            handler,
        );
        assert!(addresses.is_empty());
        assert!(commands.is_empty());
    }
}
