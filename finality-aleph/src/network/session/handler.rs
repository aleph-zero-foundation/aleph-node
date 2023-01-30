use std::collections::HashMap;

use codec::Encode;

use crate::{
    abft::NodeCount,
    crypto::{AuthorityPen, AuthorityVerifier},
    network::{
        session::{AuthData, Authentication},
        AddressingInformation,
    },
    NodeIndex, SessionId,
};

#[derive(Debug)]
pub enum SessionInfo<A: AddressingInformation> {
    SessionId(SessionId),
    OwnAuthentication(Authentication<A>),
}

impl<A: AddressingInformation> SessionInfo<A> {
    fn session_id(&self) -> SessionId {
        match self {
            SessionInfo::SessionId(session_id) => *session_id,
            SessionInfo::OwnAuthentication(peer_authentications) => {
                peer_authentications.session_id()
            }
        }
    }
}

/// A struct for handling authentications for a given session and maintaining
/// mappings between PeerIds and NodeIndexes within that session.
pub struct Handler<A: AddressingInformation> {
    peers_by_node: HashMap<NodeIndex, A::PeerId>,
    authentications: HashMap<A::PeerId, Authentication<A>>,
    session_info: SessionInfo<A>,
    own_peer_id: A::PeerId,
    authority_index_and_pen: Option<(NodeIndex, AuthorityPen)>,
    authority_verifier: AuthorityVerifier,
}

#[derive(Debug)]
pub enum HandlerError {
    /// Returned when handler is change from validator to nonvalidator
    /// or vice versa
    TypeChange,
}

async fn construct_session_info<A: AddressingInformation>(
    authority_index_and_pen: &Option<(NodeIndex, AuthorityPen)>,
    session_id: SessionId,
    address: A,
) -> (SessionInfo<A>, A::PeerId) {
    let peer_id = address.peer_id();
    match authority_index_and_pen {
        Some((node_index, authority_pen)) => {
            let auth_data = AuthData {
                address,
                node_id: *node_index,
                session_id,
            };
            let signature = authority_pen.sign(&auth_data.encode()).await;
            let authentications = Authentication(auth_data, signature);
            (SessionInfo::OwnAuthentication(authentications), peer_id)
        }
        None => (SessionInfo::SessionId(session_id), peer_id),
    }
}

impl<A: AddressingInformation> Handler<A> {
    /// Creates a new session handler. It will be a validator session handler if the authority
    /// index and pen are provided.
    pub async fn new(
        authority_index_and_pen: Option<(NodeIndex, AuthorityPen)>,
        authority_verifier: AuthorityVerifier,
        session_id: SessionId,
        address: A,
    ) -> Handler<A> {
        let (session_info, own_peer_id) =
            construct_session_info(&authority_index_and_pen, session_id, address).await;
        Handler {
            peers_by_node: HashMap::new(),
            authentications: HashMap::new(),
            session_info,
            authority_index_and_pen,
            authority_verifier,
            own_peer_id,
        }
    }

    fn index(&self) -> Option<NodeIndex> {
        match self.authority_index_and_pen {
            Some((index, _)) => Some(index),
            _ => None,
        }
    }

    pub fn is_validator(&self) -> bool {
        self.authority_index_and_pen.is_some()
    }

    pub fn node_count(&self) -> NodeCount {
        self.authority_verifier.node_count()
    }

    pub fn session_id(&self) -> SessionId {
        self.session_info.session_id()
    }

    /// Returns the authentication for the node and session this handler is responsible for.
    pub fn authentication(&self) -> Option<Authentication<A>> {
        match &self.session_info {
            SessionInfo::SessionId(_) => None,
            SessionInfo::OwnAuthentication(own_authentications) => {
                Some(own_authentications.clone())
            }
        }
    }

    /// Returns a vector of indices of nodes for which the handler has no authentication.
    pub fn missing_nodes(&self) -> Vec<NodeIndex> {
        let node_count = self.node_count().0;
        if self.peers_by_node.len() + 1 == node_count {
            return Vec::new();
        }
        (0..node_count)
            .map(NodeIndex)
            .filter(|node_id| {
                Some(*node_id) != self.index() && !self.peers_by_node.contains_key(node_id)
            })
            .collect()
    }

    /// Verifies the authentication, uses it to update mappings, and returns the address we
    /// should stay connected to if any.
    pub fn handle_authentication(&mut self, authentication: Authentication<A>) -> Option<A> {
        if authentication.0.session() != self.session_id() {
            return None;
        }
        let Authentication(auth_data, signature) = &authentication;

        let address = auth_data.address();
        if !address.verify() {
            return None;
        }
        let peer_id = address.peer_id();
        if peer_id == self.own_peer_id {
            return None;
        }
        if !self
            .authority_verifier
            .verify(&auth_data.encode(), signature, auth_data.creator())
        {
            return None;
        }
        self.peers_by_node
            .insert(auth_data.creator(), peer_id.clone());
        self.authentications.insert(peer_id, authentication);
        Some(address)
    }

    /// Returns the PeerId of the node with the given NodeIndex, if known.
    pub fn peer_id(&self, node_id: &NodeIndex) -> Option<A::PeerId> {
        self.peers_by_node.get(node_id).cloned()
    }

    /// Returns maping from NodeIndex to PeerId
    pub fn peers(&self) -> HashMap<NodeIndex, A::PeerId> {
        self.peers_by_node.clone()
    }

    /// Updates the handler with the given keychain and set of own addresses.
    /// Returns an error if the set of addresses is not valid.
    /// All authentications will be rechecked, invalid ones purged.
    /// Own authentication will be regenerated.
    /// If successful returns a set of addresses that we should be connected to.
    pub async fn update(
        &mut self,
        authority_index_and_pen: Option<(NodeIndex, AuthorityPen)>,
        authority_verifier: AuthorityVerifier,
        address: A,
    ) -> Result<Vec<A>, HandlerError> {
        if authority_index_and_pen.is_none() != self.authority_index_and_pen.is_none() {
            return Err(HandlerError::TypeChange);
        }

        let authentications = self.authentications.clone();

        *self = Handler::new(
            authority_index_and_pen,
            authority_verifier,
            self.session_id(),
            address,
        )
        .await;

        for (_, authentication) in authentications {
            self.handle_authentication(authentication);
        }
        Ok(self
            .authentications
            .values()
            .map(|authentication| authentication.0.address())
            .collect())
    }
}

#[cfg(test)]
pub mod tests {
    use super::{Handler, HandlerError};
    use crate::{
        network::{
            clique::mock::{random_address, random_invalid_address, MockAddressingInformation},
            mock::crypto_basics,
            session::Authentication,
            AddressingInformation,
        },
        NodeIndex, SessionId,
    };

    pub fn authentication(
        handler: &Handler<MockAddressingInformation>,
    ) -> Authentication<MockAddressingInformation> {
        handler
            .authentication()
            .expect("this is a validator handler")
    }

    const NUM_NODES: usize = 7;

    #[tokio::test]
    async fn identifies_whether_node_is_authority_in_current_session() {
        let mut crypto_basics = crypto_basics(NUM_NODES).await;
        let no_authority_handler = Handler::new(
            None,
            crypto_basics.1.clone(),
            SessionId(43),
            random_address(),
        )
        .await;
        let authority_handler = Handler::new(
            Some(crypto_basics.0.pop().unwrap()),
            crypto_basics.1,
            SessionId(43),
            random_address(),
        )
        .await;
        assert!(!no_authority_handler.is_validator());
        assert!(authority_handler.is_validator());
    }

    #[tokio::test]
    async fn non_validator_handler_returns_none_for_authentication() {
        let crypto_basics = crypto_basics(NUM_NODES).await;
        assert!(
            Handler::new(None, crypto_basics.1, SessionId(43), random_address(),)
                .await
                .authentication()
                .is_none()
        );
    }

    #[tokio::test]
    async fn fails_to_update_from_validator_to_non_validator() {
        let mut crypto_basics = crypto_basics(NUM_NODES).await;
        let address = random_address();
        let mut handler0 = Handler::new(
            Some(crypto_basics.0.pop().unwrap()),
            crypto_basics.1.clone(),
            SessionId(43),
            address.clone(),
        )
        .await;
        assert!(matches!(
            handler0
                .update(None, crypto_basics.1.clone(), address)
                .await,
            Err(HandlerError::TypeChange)
        ));
    }

    #[tokio::test]
    async fn fails_to_update_from_non_validator_to_validator() {
        let mut crypto_basics = crypto_basics(NUM_NODES).await;
        let address = random_address();
        let mut handler0 = Handler::new(
            None,
            crypto_basics.1.clone(),
            SessionId(43),
            address.clone(),
        )
        .await;
        assert!(matches!(
            handler0
                .update(
                    Some(crypto_basics.0.pop().unwrap()),
                    crypto_basics.1.clone(),
                    address,
                )
                .await,
            Err(HandlerError::TypeChange)
        ));
    }

    #[tokio::test]
    async fn does_not_keep_own_peer_id_or_authentication() {
        let mut crypto_basics = crypto_basics(NUM_NODES).await;
        let handler0 = Handler::new(
            Some(crypto_basics.0.pop().unwrap()),
            crypto_basics.1,
            SessionId(43),
            random_address(),
        )
        .await;
        assert!(handler0.peer_id(&NodeIndex(0)).is_none());
    }

    #[tokio::test]
    async fn misses_all_other_nodes_initially() {
        let mut crypto_basics = crypto_basics(NUM_NODES).await;
        let handler0 = Handler::new(
            Some(crypto_basics.0.pop().unwrap()),
            crypto_basics.1,
            SessionId(43),
            random_address(),
        )
        .await;
        let missing_nodes = handler0.missing_nodes();
        let expected_missing: Vec<_> = (0..NUM_NODES - 1).map(NodeIndex).collect();
        assert_eq!(missing_nodes, expected_missing);
        assert!(handler0.peer_id(&NodeIndex(1)).is_none());
    }

    #[tokio::test]
    async fn accepts_correct_authentication() {
        let crypto_basics = crypto_basics(NUM_NODES).await;
        let mut handler0 = Handler::new(
            Some(crypto_basics.0[0].clone()),
            crypto_basics.1.clone(),
            SessionId(43),
            random_address(),
        )
        .await;
        let address = random_address();
        let handler1 = Handler::new(
            Some(crypto_basics.0[1].clone()),
            crypto_basics.1.clone(),
            SessionId(43),
            address.clone(),
        )
        .await;
        assert!(handler0
            .handle_authentication(authentication(&handler1))
            .is_some());
        let missing_nodes = handler0.missing_nodes();
        let expected_missing: Vec<_> = (2..NUM_NODES).map(NodeIndex).collect();
        assert_eq!(missing_nodes, expected_missing);
        let peer_id1 = address.peer_id();
        assert_eq!(handler0.peer_id(&NodeIndex(1)), Some(peer_id1));
    }

    #[tokio::test]
    async fn non_validator_accepts_correct_authentication() {
        let crypto_basics = crypto_basics(NUM_NODES).await;
        let mut handler0 = Handler::new(
            None,
            crypto_basics.1.clone(),
            SessionId(43),
            random_address(),
        )
        .await;
        let address = random_address();
        let handler1 = Handler::new(
            Some(crypto_basics.0[1].clone()),
            crypto_basics.1.clone(),
            SessionId(43),
            address.clone(),
        )
        .await;
        assert!(handler0
            .handle_authentication(authentication(&handler1))
            .is_some());
        let missing_nodes = handler0.missing_nodes();
        let mut expected_missing: Vec<_> = (0..NUM_NODES).map(NodeIndex).collect();
        expected_missing.remove(1);
        assert_eq!(missing_nodes, expected_missing);
        let peer_id1 = address.peer_id();
        assert_eq!(handler0.peer_id(&NodeIndex(1)), Some(peer_id1));
    }

    #[tokio::test]
    async fn ignores_invalid_authentication() {
        let crypto_basics = crypto_basics(NUM_NODES).await;
        let mut handler0 = Handler::new(
            Some(crypto_basics.0[0].clone()),
            crypto_basics.1.clone(),
            SessionId(43),
            random_address(),
        )
        .await;
        let handler1 = Handler::new(
            Some(crypto_basics.0[1].clone()),
            crypto_basics.1.clone(),
            SessionId(43),
            random_invalid_address(),
        )
        .await;
        assert!(handler0
            .handle_authentication(authentication(&handler1))
            .is_none());
        let missing_nodes = handler0.missing_nodes();
        let expected_missing: Vec<_> = (1..NUM_NODES).map(NodeIndex).collect();
        assert_eq!(missing_nodes, expected_missing);
    }

    #[tokio::test]
    async fn ignores_badly_signed_authentication() {
        let crypto_basics = crypto_basics(NUM_NODES).await;
        let mut handler0 = Handler::new(
            Some(crypto_basics.0[0].clone()),
            crypto_basics.1.clone(),
            SessionId(43),
            random_address(),
        )
        .await;
        let handler1 = Handler::new(
            Some(crypto_basics.0[1].clone()),
            crypto_basics.1.clone(),
            SessionId(43),
            random_address(),
        )
        .await;
        let mut bad_authentication = authentication(&handler1);
        bad_authentication.1 = authentication(&handler0).1;
        assert!(handler0.handle_authentication(bad_authentication).is_none());
        let missing_nodes = handler0.missing_nodes();
        let expected_missing: Vec<_> = (1..NUM_NODES).map(NodeIndex).collect();
        assert_eq!(missing_nodes, expected_missing);
    }

    #[tokio::test]
    async fn ignores_wrong_session_authentication() {
        let crypto_basics = crypto_basics(NUM_NODES).await;
        let mut handler0 = Handler::new(
            Some(crypto_basics.0[0].clone()),
            crypto_basics.1.clone(),
            SessionId(43),
            random_address(),
        )
        .await;
        let handler1 = Handler::new(
            Some(crypto_basics.0[1].clone()),
            crypto_basics.1.clone(),
            SessionId(44),
            random_address(),
        )
        .await;
        assert!(handler0
            .handle_authentication(authentication(&handler1))
            .is_none());
        let missing_nodes = handler0.missing_nodes();
        let expected_missing: Vec<_> = (1..NUM_NODES).map(NodeIndex).collect();
        assert_eq!(missing_nodes, expected_missing);
    }

    #[tokio::test]
    async fn ignores_own_authentication() {
        let awaited_crypto_basics = crypto_basics(NUM_NODES).await;
        let mut handler0 = Handler::new(
            Some(awaited_crypto_basics.0[0].clone()),
            awaited_crypto_basics.1.clone(),
            SessionId(43),
            random_address(),
        )
        .await;
        assert!(handler0
            .handle_authentication(authentication(&handler0))
            .is_none());
        let missing_nodes = handler0.missing_nodes();
        let expected_missing: Vec<_> = (1..NUM_NODES).map(NodeIndex).collect();
        assert_eq!(missing_nodes, expected_missing);
    }

    #[tokio::test]
    async fn invalidates_obsolete_authentication() {
        let awaited_crypto_basics = crypto_basics(NUM_NODES).await;
        let mut handler0 = Handler::new(
            Some(awaited_crypto_basics.0[0].clone()),
            awaited_crypto_basics.1.clone(),
            SessionId(43),
            random_address(),
        )
        .await;
        let handler1 = Handler::new(
            Some(awaited_crypto_basics.0[1].clone()),
            awaited_crypto_basics.1.clone(),
            SessionId(43),
            random_address(),
        )
        .await;
        assert!(handler0
            .handle_authentication(authentication(&handler1))
            .is_some());
        let new_crypto_basics = crypto_basics(NUM_NODES).await;
        handler0
            .update(
                Some(new_crypto_basics.0[0].clone()),
                new_crypto_basics.1.clone(),
                random_address(),
            )
            .await
            .unwrap();
        let missing_nodes = handler0.missing_nodes();
        let expected_missing: Vec<_> = (1..NUM_NODES).map(NodeIndex).collect();
        assert_eq!(missing_nodes, expected_missing);
        assert!(handler0.peer_id(&NodeIndex(1)).is_none());
    }
}
