use crate::{
    new_network::{
        connection_manager::{Authentication, Multiaddr, SessionHandler},
        DataCommand, PeerId, Protocol,
    },
    NodeCount, NodeIndex, SessionId,
};
use codec::{Decode, Encode};
use log::warn;
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

/// How many nodes we should query about unknown authorities in one go.
const NODES_TO_QUERY: usize = 2;

/// Messages used for discovery and authentication.
#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub enum DiscoveryMessage {
    AuthenticationBroadcast(Authentication),
    // Requests always contain own authentication, to avoid asymmetric trust.
    Request(Vec<NodeIndex>, Authentication),
    // Always assumed to contain only authentications for one session.
    // Only authentications from the same session as the first present are guaranteed to be
    // processed.
    Authentications(Vec<Authentication>),
}

impl DiscoveryMessage {
    pub fn session_id(&self) -> SessionId {
        use DiscoveryMessage::*;
        match self {
            AuthenticationBroadcast((auth_data, _)) => auth_data.session(),
            Request(_, (auth_data, _)) => auth_data.session(),
            Authentications(auths) => match auths.get(0) {
                Some((auth_data, _)) => auth_data.session(),
                None => SessionId(0), // Broken message anyway, value doesn't matter.
            },
        }
    }
}

/// Handles creating and responding to discovery messages.
pub struct Discovery {
    cooldown: Duration,
    last_broadcast: HashMap<NodeIndex, Instant>,
    last_response: HashMap<NodeIndex, Instant>,
    requested_authentications: HashMap<NodeIndex, HashSet<NodeIndex>>,
    // Used to rotate the nodes we query about unknown nodes.
    next_query: usize,
}

type DiscoveryCommand = (DiscoveryMessage, DataCommand);

fn authentication_broadcast(authentication: Authentication) -> DiscoveryCommand {
    (
        DiscoveryMessage::AuthenticationBroadcast(authentication),
        DataCommand::Broadcast,
    )
}

fn request(
    missing_authorities: Vec<NodeIndex>,
    authentication: Authentication,
    peer_id: PeerId,
) -> DiscoveryCommand {
    (
        DiscoveryMessage::Request(missing_authorities, authentication),
        DataCommand::SendTo(peer_id, Protocol::Generic),
    )
}

fn response(authentications: Vec<Authentication>, peer_id: PeerId) -> DiscoveryCommand {
    (
        DiscoveryMessage::Authentications(authentications),
        DataCommand::SendTo(peer_id, Protocol::Generic),
    )
}

impl Discovery {
    /// Create a new discovery handler with the given response/broadcast cooldown.
    pub fn new(cooldown: Duration) -> Self {
        Discovery {
            cooldown,
            last_broadcast: HashMap::new(),
            last_response: HashMap::new(),
            requested_authentications: HashMap::new(),
            next_query: rand::random(),
        }
    }

    fn should_broadcast(missing_authorities_num: usize, total_node_count: NodeCount) -> bool {
        // If we are not sure we know of at least one honest node.
        missing_authorities_num * 3 > 2 * total_node_count.0
    }

    /// Returns messages that should be sent as part of authority discovery at this moment.
    pub fn discover_authorities(&mut self, handler: &SessionHandler) -> Vec<DiscoveryCommand> {
        let missing_authorities = handler.missing_nodes();
        if missing_authorities.is_empty() {
            return Vec::new();
        }
        let node_count = handler.node_count();
        let authentication = handler.authentication();
        if Self::should_broadcast(missing_authorities.len(), node_count) {
            // We know of fewer than 1/3 authorities, broadcast our authentication and hope others
            // respond in kind.
            vec![authentication_broadcast(authentication)]
        } else {
            // Attempt learning about more authorities from the ones you already know.
            let mut result = Vec::new();
            let mut target = NodeIndex(self.next_query % node_count.0);
            while result.len() < NODES_TO_QUERY {
                if let Some(peer_id) = handler.peer_id(&target) {
                    result.push(request(
                        missing_authorities.clone(),
                        authentication.clone(),
                        peer_id,
                    ));
                }
                target = NodeIndex((target.0 + 1) % node_count.0);
            }
            self.next_query = target.0;
            result
        }
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
        let addresses = self.handle_authentication(authentication.clone(), handler);
        if addresses.is_empty() {
            return (Vec::new(), Vec::new());
        }
        let node_id = authentication.0.creator();
        let mut messages = Vec::new();
        match handler.peer_id(&node_id) {
            Some(peer_id) => messages.push(response(vec![handler.authentication()], peer_id)),
            None => {
                warn!(target: "aleph-network", "Id of correctly authenticated peer not present.")
            }
        }
        if self.should_rebroadcast(&node_id) {
            self.last_broadcast.insert(node_id, Instant::now());
            messages.push(authentication_broadcast(authentication));
        }
        (addresses, messages)
    }

    fn create_response(
        &mut self,
        requester_id: NodeIndex,
        node_ids: Vec<NodeIndex>,
        handler: &mut SessionHandler,
    ) -> Vec<DiscoveryCommand> {
        let requested_authentications = self
            .requested_authentications
            .entry(requester_id)
            .or_default();
        requested_authentications.extend(
            node_ids
                .iter()
                .filter(|n_id| n_id.0 < handler.node_count().0),
        );
        if let Some(instant) = self.last_response.get(&requester_id) {
            if Instant::now() < *instant + self.cooldown {
                return Vec::new();
            }
        }
        let peer_id = match handler.peer_id(&requester_id) {
            Some(peer_id) => peer_id,
            None => return Vec::new(),
        };
        let authentications: Vec<_> = requested_authentications
            .iter()
            .filter_map(|id| handler.authentication_for(id))
            .collect();
        if authentications.is_empty() {
            Vec::new()
        } else {
            self.last_response.insert(requester_id, Instant::now());
            self.requested_authentications.remove(&requester_id);
            vec![response(authentications, peer_id)]
        }
    }

    fn handle_request(
        &mut self,
        node_ids: Vec<NodeIndex>,
        authentication: Authentication,
        handler: &mut SessionHandler,
    ) -> (Vec<Multiaddr>, Vec<DiscoveryCommand>) {
        let node_id = authentication.0.creator();
        let addresses = self.handle_authentication(authentication, handler);
        if addresses.is_empty() {
            return (Vec::new(), Vec::new());
        }
        (addresses, self.create_response(node_id, node_ids, handler))
    }

    fn handle_response(
        &mut self,
        authentications: Vec<Authentication>,
        handler: &mut SessionHandler,
    ) -> Vec<Multiaddr> {
        authentications
            .into_iter()
            .flat_map(|authentication| self.handle_authentication(authentication, handler))
            .collect()
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
            Request(node_ids, authentication) => {
                self.handle_request(node_ids, authentication, handler)
            }
            Authentications(authentications) => {
                (self.handle_response(authentications, handler), Vec::new())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Discovery, DiscoveryMessage};
    use crate::{
        crypto::{AuthorityPen, AuthorityVerifier, KeyBox},
        new_network::{
            connection_manager::{Authentication, SessionHandler},
            DataCommand, Multiaddr, Protocol,
        },
        NodeIndex, SessionId,
    };
    use aleph_primitives::{AuthorityId, KEY_TYPE};
    use codec::Encode;
    use sc_network::{
        multiaddr::Protocol as ScProtocol, Multiaddr as ScMultiaddr, PeerId as ScPeerId,
    };
    use sp_keystore::{testing::KeyStore, CryptoStore};
    use std::{
        collections::HashSet, iter, net::Ipv4Addr, sync::Arc, thread::sleep, time::Duration,
    };

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

    async fn keyboxes() -> Vec<KeyBox> {
        let num_keyboxes: usize = NUM_NODES.into();
        let keystore = Arc::new(KeyStore::new());
        let mut auth_ids = Vec::with_capacity(num_keyboxes);
        for _ in 0..num_keyboxes {
            let pk = keystore.ed25519_generate_new(KEY_TYPE, None).await.unwrap();
            auth_ids.push(AuthorityId::from(pk));
        }
        let mut result = Vec::with_capacity(num_keyboxes);
        for i in 0..num_keyboxes {
            result.push(KeyBox::new(
                NodeIndex(i),
                AuthorityVerifier::new(auth_ids.clone()),
                AuthorityPen::new(auth_ids[i].clone(), keystore.clone())
                    .await
                    .expect("The keys should sign successfully"),
            ));
        }
        result
    }

    async fn build() -> (Discovery, Vec<SessionHandler>) {
        let mut handlers = Vec::new();
        for (keybox, address) in keyboxes().await.into_iter().zip(addresses()) {
            handlers.push(
                SessionHandler::new(keybox, SessionId(43), vec![address.into()])
                    .await
                    .unwrap(),
            );
        }
        (Discovery::new(Duration::from_millis(MS_COOLDOWN)), handlers)
    }

    #[tokio::test]
    async fn broadcasts_when_clueless() {
        let (mut discovery, mut handlers) = build().await;
        let handler = &mut handlers[0];
        let mut messages = discovery.discover_authorities(handler);
        assert_eq!(messages.len(), 1);
        let message = messages.pop().unwrap();
        assert_eq!(
            message,
            (
                DiscoveryMessage::AuthenticationBroadcast(handler.authentication()),
                DataCommand::Broadcast
            )
        );
    }

    #[tokio::test]
    async fn requests_from_single_when_only_some_missing() {
        let num_nodes: usize = NUM_NODES.into();
        let (mut discovery, mut handlers) = build().await;
        for i in 1..num_nodes - 1 {
            let authentication = handlers[i].authentication();
            assert!(handlers[0].handle_authentication(authentication));
        }
        let handler = &mut handlers[0];
        let messages = discovery.discover_authorities(handler);
        assert_eq!(messages.len(), 2);
        for message in messages {
            assert!(matches!(message,(
                        DiscoveryMessage::Request(node_ids, authentication),
                        DataCommand::SendTo(_, _),
                    ) if node_ids == vec![NodeIndex(6)]
                        && authentication == handler.authentication()));
        }
    }

    #[tokio::test]
    async fn requests_nothing_when_knows_all() {
        let num_nodes: usize = NUM_NODES.into();
        let (mut discovery, mut handlers) = build().await;
        for i in 1..num_nodes {
            let authentication = handlers[i].authentication();
            assert!(handlers[0].handle_authentication(authentication));
        }
        let handler = &mut handlers[0];
        let messages = discovery.discover_authorities(handler);
        assert!(messages.is_empty());
    }

    #[tokio::test]
    async fn rebroadcasts_responds_and_accepts_addresses() {
        let (mut discovery, mut handlers) = build().await;
        let authentication = handlers[1].authentication();
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
                DiscoveryMessage::Authentications(authentications),
                DataCommand::SendTo(_, _),
            ) if authentications == &vec![handler.authentication()])));
    }

    #[tokio::test]
    async fn does_not_rebroadcast_nor_respond_to_wrong_authentications() {
        let (mut discovery, mut handlers) = build().await;
        let (auth_data, _) = handlers[1].authentication();
        let (_, signature) = handlers[2].authentication();
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
        let (mut discovery, mut handlers) = build().await;
        let authentication = handlers[1].authentication();
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
                DiscoveryMessage::Authentications(authentications),
                DataCommand::SendTo(_, _),
            ) if authentications == &vec![handler.authentication()]));
    }

    #[tokio::test]
    async fn rebroadcasts_after_cooldown() {
        let (mut discovery, mut handlers) = build().await;
        let authentication = handlers[1].authentication();
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
    async fn responds_to_correct_request_when_can() {
        let (mut discovery, mut handlers) = build().await;
        let requested_authentication = handlers[1].authentication();
        let requested_node_id = requested_authentication.0.creator();
        let requester_authentication = handlers[2].authentication();
        let handler = &mut handlers[0];
        assert!(handler.handle_authentication(requested_authentication.clone()));
        let (addresses, commands) = discovery.handle_message(
            DiscoveryMessage::Request(vec![requested_node_id], requester_authentication.clone()),
            handler,
        );
        assert_eq!(addresses, requester_authentication.0.addresses());
        assert_eq!(commands.len(), 1);
        assert!(matches!(&commands[0], (
                DiscoveryMessage::Authentications(response_authentications),
                DataCommand::SendTo(peer_id, Protocol::Generic),
            ) if Some(*peer_id) == handler.peer_id(&requester_authentication.0.creator())
                && response_authentications == &vec![requested_authentication]));
    }

    #[tokio::test]
    async fn does_not_respond_to_correct_request_when_cannot() {
        let (mut discovery, mut handlers) = build().await;
        let requested_node_id = NodeIndex(1);
        let requester_authentication = handlers[2].authentication();
        let handler = &mut handlers[0];
        let (addresses, commands) = discovery.handle_message(
            DiscoveryMessage::Request(vec![requested_node_id], requester_authentication.clone()),
            handler,
        );
        assert_eq!(addresses, requester_authentication.0.addresses());
        assert!(commands.is_empty())
    }

    #[tokio::test]
    async fn does_not_respond_to_incorrect_request() {
        let (mut discovery, mut handlers) = build().await;
        let requested_authentication = handlers[1].authentication();
        let requested_node_id = requested_authentication.0.creator();
        let (auth_data, _) = handlers[2].authentication();
        let (_, signature) = handlers[3].authentication();
        let requester_authentication = (auth_data, signature);
        let handler = &mut handlers[0];
        let (addresses, commands) = discovery.handle_message(
            DiscoveryMessage::Request(vec![requested_node_id], requester_authentication),
            handler,
        );
        assert!(addresses.is_empty());
        assert!(commands.is_empty());
    }

    #[tokio::test]
    async fn does_not_respond_too_quickly() {
        let (mut discovery, mut handlers) = build().await;
        let requested_authentication = handlers[1].authentication();
        let requested_node_id = requested_authentication.0.creator();
        let requester_authentication = handlers[2].authentication();
        let handler = &mut handlers[0];
        assert!(handler.handle_authentication(requested_authentication.clone()));
        let (addresses, commands) = discovery.handle_message(
            DiscoveryMessage::Request(vec![requested_node_id], requester_authentication.clone()),
            handler,
        );
        assert_eq!(addresses, requester_authentication.0.addresses());
        assert_eq!(commands.len(), 1);
        assert!(matches!(&commands[0], (
                DiscoveryMessage::Authentications(response_authentications),
                DataCommand::SendTo(peer_id, Protocol::Generic),
            ) if Some(*peer_id) == handler.peer_id(&requester_authentication.0.creator())
                && response_authentications == &vec![requested_authentication]));
        let (addresses, commands) = discovery.handle_message(
            DiscoveryMessage::Request(vec![requested_node_id], requester_authentication.clone()),
            handler,
        );
        assert_eq!(addresses, requester_authentication.0.addresses());
        assert!(commands.is_empty());
    }

    #[tokio::test]
    async fn responds_cumulatively_after_cooldown() {
        let (mut discovery, mut handlers) = build().await;
        let requester_authentication = handlers[1].authentication();
        let available_authentications_start: usize = 2;
        let available_authentications_end: usize = (NUM_NODES - 2).into();
        let available_authentications: Vec<Authentication> = (available_authentications_start
            ..available_authentications_end)
            .map(|i| handlers[i].authentication())
            .collect();
        let handler = &mut handlers[0];
        for authentication in &available_authentications {
            assert!(handler.handle_authentication(authentication.clone()));
        }
        let requested_node_id = NodeIndex(2);
        let (addresses, commands) = discovery.handle_message(
            DiscoveryMessage::Request(vec![requested_node_id], requester_authentication.clone()),
            handler,
        );
        assert_eq!(addresses, requester_authentication.0.addresses());
        assert_eq!(commands.len(), 1);
        assert!(matches!(&commands[0], (
                DiscoveryMessage::Authentications(response_authentications),
                DataCommand::SendTo(peer_id, Protocol::Generic),
            ) if Some(*peer_id) == handler.peer_id(&requester_authentication.0.creator())
                && response_authentications == &vec![available_authentications[0].clone()]));
        let requested_node_id = NodeIndex(3);
        let (_, commands) = discovery.handle_message(
            DiscoveryMessage::Request(vec![requested_node_id], requester_authentication.clone()),
            handler,
        );
        assert!(commands.is_empty());
        sleep(Duration::from_millis(MS_COOLDOWN + 5));
        let requested_node_id = NodeIndex(available_authentications_end);
        let (_, commands) = discovery.handle_message(
            DiscoveryMessage::Request(vec![requested_node_id], requester_authentication.clone()),
            handler,
        );
        assert_eq!(commands.len(), 1);
        assert!(matches!(&commands[0], (
                DiscoveryMessage::Authentications(response_authentications),
                DataCommand::SendTo(peer_id, Protocol::Generic),
            ) if Some(*peer_id) == handler.peer_id(&requester_authentication.0.creator())
                && response_authentications == &vec![available_authentications[1].clone()]));
    }

    #[tokio::test]
    async fn accepts_correct_authentications() {
        let (mut discovery, mut handlers) = build().await;
        let authentications_start: usize = 1;
        let authentications_end: usize = (NUM_NODES - 2).into();
        let authentications =
            (authentications_start..authentications_end).map(|i| handlers[i].authentication());
        let expected_addresses: HashSet<_> = authentications
            .clone()
            .flat_map(|(auth_data, _)| auth_data.addresses())
            .map(|address| address.encode())
            .collect();
        let authentications = authentications.collect();
        let handler = &mut handlers[0];
        let (addresses, commands) =
            discovery.handle_message(DiscoveryMessage::Authentications(authentications), handler);
        let addresses: HashSet<_> = addresses
            .into_iter()
            .map(|address| address.encode())
            .collect();
        assert_eq!(addresses, expected_addresses);
        assert!(commands.is_empty());
    }

    #[tokio::test]
    async fn does_not_accept_incorrect_authentications() {
        let (mut discovery, mut handlers) = build().await;
        let authentications_start: usize = 1;
        let authentications_end: usize = (NUM_NODES - 2).into();
        let authentications =
            (authentications_start..authentications_end).map(|i| handlers[i].authentication());
        let (auth_data, _) = handlers[authentications_end].authentication();
        let (_, signature) = handlers[authentications_end - 1].authentication();
        let incorrect_authentication = (auth_data, signature);
        let expected_addresses: HashSet<_> = authentications
            .clone()
            .flat_map(|(auth_data, _)| auth_data.addresses())
            .map(|address| address.encode())
            .collect();
        let authentications = iter::once(incorrect_authentication)
            .chain(authentications)
            .collect();
        let handler = &mut handlers[0];
        let (addresses, commands) =
            discovery.handle_message(DiscoveryMessage::Authentications(authentications), handler);
        let addresses: HashSet<_> = addresses
            .into_iter()
            .map(|address| address.encode())
            .collect();
        assert_eq!(addresses, expected_addresses);
        assert!(commands.is_empty());
    }
}
