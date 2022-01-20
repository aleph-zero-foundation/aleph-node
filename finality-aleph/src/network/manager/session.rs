use crate::{
    crypto::{AuthorityPen, AuthorityVerifier},
    network::{
        manager::{get_common_peer_id, is_p2p, AuthData, Authentication, Multiaddr},
        PeerId,
    },
    NodeIndex, SessionId,
};
use aleph_bft::NodeCount;
use codec::Encode;
use std::collections::HashMap;

#[derive(Debug)]
pub enum SessionInfo {
    SessionId(SessionId),
    OwnAuthentication(Authentication),
}

impl SessionInfo {
    fn session_id(&self) -> SessionId {
        match self {
            SessionInfo::SessionId(session_id) => *session_id,
            SessionInfo::OwnAuthentication((auth_data, _)) => auth_data.session_id,
        }
    }
}

/// A struct for handling authentications for a given session and maintaining
/// mappings between PeerIds and NodeIndexes within that session.
pub struct Handler {
    peers_by_node: HashMap<NodeIndex, PeerId>,
    authentications: HashMap<PeerId, (Authentication, Option<Authentication>)>,
    session_info: SessionInfo,
    own_peer_id: PeerId,
    authority_index_and_pen: Option<(NodeIndex, AuthorityPen)>,
    authority_verifier: AuthorityVerifier,
}

#[derive(Debug)]
pub enum HandlerError {
    /// Returned when handler is change from validator to nonvalidator
    /// or vice versa
    TypeChange,
    /// Returned when a set of addresses is not usable for creating authentications.
    /// Either because none of the addresses are externally reachable libp2p addresses,
    /// or the addresses contain multiple libp2p PeerIds.
    NoP2pAddresses,
    MultiplePeerIds,
}

fn retrieve_peer_id(addresses: &[Multiaddr]) -> Result<PeerId, HandlerError> {
    if addresses.is_empty() {
        return Err(HandlerError::NoP2pAddresses);
    }
    get_common_peer_id(addresses).ok_or(HandlerError::MultiplePeerIds)
}

async fn construct_session_info(
    authority_index_and_pen: &Option<(NodeIndex, AuthorityPen)>,
    session_id: SessionId,
    addresses: Vec<Multiaddr>,
) -> Result<(SessionInfo, PeerId), HandlerError> {
    let addresses: Vec<_> = addresses.into_iter().filter(is_p2p).collect();
    let peer = retrieve_peer_id(&addresses)?;

    if let Some((node_index, authority_pen)) = authority_index_and_pen {
        let auth_data = AuthData {
            addresses,
            node_id: *node_index,
            session_id,
        };
        let signature = authority_pen.sign(&auth_data.encode()).await;
        return Ok((SessionInfo::OwnAuthentication((auth_data, signature)), peer));
    }
    Ok((SessionInfo::SessionId(session_id), peer))
}

impl Handler {
    /// Returns an error if the set of addresses contains no external libp2p addresses, or contains
    /// at least two such addresses with differing PeerIds.
    pub async fn new(
        authority_index_and_pen: Option<(NodeIndex, AuthorityPen)>,
        authority_verifier: AuthorityVerifier,
        session_id: SessionId,
        addresses: Vec<Multiaddr>,
    ) -> Result<Handler, HandlerError> {
        let (session_info, own_peer_id) =
            construct_session_info(&authority_index_and_pen, session_id, addresses).await?;
        Ok(Handler {
            peers_by_node: HashMap::new(),
            authentications: HashMap::new(),
            session_info,
            authority_index_and_pen,
            authority_verifier,
            own_peer_id,
        })
    }

    fn index(&self) -> Option<NodeIndex> {
        match self.authority_index_and_pen {
            Some((index, _)) => Some(index),
            _ => None,
        }
    }

    pub fn node_count(&self) -> NodeCount {
        self.authority_verifier.node_count()
    }

    fn session_id(&self) -> SessionId {
        self.session_info.session_id()
    }

    /// Returns the authentication for the node and session this handler is responsible for.
    pub fn authentication(&self) -> Option<Authentication> {
        match &self.session_info {
            SessionInfo::SessionId(_) => None,
            SessionInfo::OwnAuthentication(own_authentication) => Some(own_authentication.clone()),
        }
    }

    /// Returns the authentication for the node with the given index, if we have it.
    pub fn authentication_for(&self, node_id: &NodeIndex) -> Option<Authentication> {
        self.peer_id(node_id)
            .map(|peer_id| self.authentications.get(&peer_id))
            .flatten()
            .map(|(auth, _)| auth.clone())
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

    /// Verifies the authentication, uses it to update mappings, and returns whether we should
    /// remain connected to the multiaddresses.
    pub fn handle_authentication(&mut self, authentication: Authentication) -> bool {
        if authentication.0.session_id != self.session_id() {
            return false;
        }
        let (auth_data, signature) = &authentication;

        // The auth is completely useless if it doesn't have a consistent PeerId.
        let peer_id = match get_common_peer_id(&auth_data.addresses) {
            Some(peer_id) => peer_id,
            None => return false,
        };
        if peer_id == self.own_peer_id {
            return false;
        }
        if !self
            .authority_verifier
            .verify(&auth_data.encode(), signature, auth_data.node_id)
        {
            // This might be an authentication for a key that has been changed, but we are not yet
            // aware of the change.
            if let Some(auth_pair) = self.authentications.get_mut(&peer_id) {
                auth_pair.1 = Some(authentication.clone());
            }
            return false;
        }
        self.peers_by_node.insert(auth_data.node_id, peer_id);
        self.authentications.insert(peer_id, (authentication, None));
        true
    }

    /// Returns the PeerId of the node with the given NodeIndex, if known.
    pub fn peer_id(&self, node_id: &NodeIndex) -> Option<PeerId> {
        self.peers_by_node.get(node_id).copied()
    }

    /// Updates the handler with the given keychain and set of own addresses.
    /// Returns an error if the set of addresses is not valid.
    /// All authentications will be rechecked, invalid ones purged and cached ones that turn out to
    /// now be valid canonalized.
    /// Own authentication will be regenerated.
    /// If successful returns a set of addresses that we should be connected to.
    pub async fn update(
        &mut self,
        authority_index_and_pen: Option<(NodeIndex, AuthorityPen)>,
        authority_verifier: AuthorityVerifier,
        addresses: Vec<Multiaddr>,
    ) -> Result<Vec<Multiaddr>, HandlerError> {
        if authority_index_and_pen.is_none() != self.authority_index_and_pen.is_none() {
            return Err(HandlerError::TypeChange);
        }

        let authentications = self.authentications.clone();

        *self = Handler::new(
            authority_index_and_pen,
            authority_verifier,
            self.session_id(),
            addresses,
        )
        .await?;

        for (_, (auth, maybe_auth)) in authentications {
            print!(
                "normal authentication: {:?}",
                self.handle_authentication(auth)
            );
            if let Some(auth) = maybe_auth {
                print!(
                    "alternative authentication: {:?}",
                    self.handle_authentication(auth)
                );
            }
        }
        Ok(self
            .authentications
            .values()
            .flat_map(|((auth_data, _), _)| auth_data.addresses.iter().cloned())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::{get_common_peer_id, Handler, HandlerError};
    use crate::{
        network::manager::{
            testing::{address, crypto_basics},
            Multiaddr,
        },
        NodeIndex, SessionId,
    };
    use codec::Encode;

    const NUM_NODES: usize = 7;

    fn correct_addresses_0() -> Vec<Multiaddr> {
        vec![
                address("/dns4/example.com/tcp/30333/p2p/12D3KooWRkGLz4YbVmrsWK75VjFTs8NvaBu42xhAmQaP4KeJpw1L").into(),
                address("/dns4/peer.example.com/tcp/30333/p2p/12D3KooWRkGLz4YbVmrsWK75VjFTs8NvaBu42xhAmQaP4KeJpw1L").into(),
        ]
    }

    fn correct_addresses_1() -> Vec<Multiaddr> {
        vec![
                address("/dns4/other.example.com/tcp/30333/p2p/12D3KooWFVXnvJdPuGnGYMPn5qLQAQYwmRBgo6SmEQsKZSrDoo2k").into(),
                address("/dns4/peer.other.example.com/tcp/30333/p2p/12D3KooWFVXnvJdPuGnGYMPn5qLQAQYwmRBgo6SmEQsKZSrDoo2k").into(),
        ]
    }

    fn local_p2p_addresses() -> Vec<Multiaddr> {
        vec![address(
            "/ip4/127.0.0.1/tcp/30333/p2p/12D3KooWFVXnvJdPuGnGYMPn5qLQAQYwmRBgo6SmEQsKZSrDoo2k",
        )
        .into()]
    }

    #[tokio::test]
    async fn creates_with_correct_data() {
        let mut crypto_basics = crypto_basics(NUM_NODES).await;
        assert!(Handler::new(
            Some(crypto_basics.0.pop().unwrap()),
            crypto_basics.1,
            SessionId(43),
            correct_addresses_0()
        )
        .await
        .is_ok());
    }

    #[tokio::test]
    async fn creates_with_local_address() {
        let mut crypto_basics = crypto_basics(NUM_NODES).await;
        assert!(Handler::new(
            Some(crypto_basics.0.pop().unwrap()),
            crypto_basics.1,
            SessionId(43),
            local_p2p_addresses()
        )
        .await
        .is_ok());
    }

    #[tokio::test]
    async fn creates_without_node_index_nor_authority_pen() {
        let crypto_basics = crypto_basics(NUM_NODES).await;
        assert!(
            Handler::new(None, crypto_basics.1, SessionId(43), correct_addresses_0())
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn non_validator_handler_returns_none_for_authentication() {
        let crypto_basics = crypto_basics(NUM_NODES).await;
        assert!(
            Handler::new(None, crypto_basics.1, SessionId(43), correct_addresses_0())
                .await
                .unwrap()
                .authentication()
                .is_none()
        );
    }

    #[tokio::test]
    async fn fails_to_create_with_no_addresses() {
        let mut crypto_basics = crypto_basics(NUM_NODES).await;
        assert!(matches!(
            Handler::new(
                Some(crypto_basics.0.pop().unwrap()),
                crypto_basics.1,
                SessionId(43),
                Vec::new()
            )
            .await,
            Err(HandlerError::NoP2pAddresses)
        ));
    }

    #[tokio::test]
    async fn fails_to_create_with_non_unique_peer_id() {
        let mut crypto_basics = crypto_basics(NUM_NODES).await;
        let addresses = correct_addresses_0()
            .into_iter()
            .chain(correct_addresses_1())
            .collect();
        assert!(matches!(
            Handler::new(
                Some(crypto_basics.0.pop().unwrap()),
                crypto_basics.1,
                SessionId(43),
                addresses
            )
            .await,
            Err(HandlerError::MultiplePeerIds)
        ));
    }

    #[tokio::test]
    async fn fails_to_update_from_validator_to_non_validator() {
        let mut crypto_basics = crypto_basics(NUM_NODES).await;
        let mut handler0 = Handler::new(
            Some(crypto_basics.0.pop().unwrap()),
            crypto_basics.1.clone(),
            SessionId(43),
            correct_addresses_0(),
        )
        .await
        .unwrap();
        assert!(matches!(
            handler0
                .update(None, crypto_basics.1.clone(), correct_addresses_0())
                .await,
            Err(HandlerError::TypeChange)
        ));
    }

    #[tokio::test]
    async fn fails_to_update_from_non_validator_to_validator() {
        let mut crypto_basics = crypto_basics(NUM_NODES).await;
        let mut handler0 = Handler::new(
            None,
            crypto_basics.1.clone(),
            SessionId(43),
            correct_addresses_0(),
        )
        .await
        .unwrap();
        assert!(matches!(
            handler0
                .update(
                    Some(crypto_basics.0.pop().unwrap()),
                    crypto_basics.1.clone(),
                    correct_addresses_0()
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
            correct_addresses_0(),
        )
        .await
        .unwrap();
        assert!(handler0.peer_id(&NodeIndex(0)).is_none());
        assert!(handler0.authentication_for(&NodeIndex(0)).is_none());
    }

    #[tokio::test]
    async fn misses_all_other_nodes_initially() {
        let mut crypto_basics = crypto_basics(NUM_NODES).await;
        let handler0 = Handler::new(
            Some(crypto_basics.0.pop().unwrap()),
            crypto_basics.1,
            SessionId(43),
            correct_addresses_0(),
        )
        .await
        .unwrap();
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
            correct_addresses_0(),
        )
        .await
        .unwrap();
        let handler1 = Handler::new(
            Some(crypto_basics.0[1].clone()),
            crypto_basics.1.clone(),
            SessionId(43),
            correct_addresses_1(),
        )
        .await
        .unwrap();
        assert!(handler0.handle_authentication(handler1.authentication().unwrap()));
        let missing_nodes = handler0.missing_nodes();
        let expected_missing: Vec<_> = (2..NUM_NODES).map(NodeIndex).collect();
        assert_eq!(missing_nodes, expected_missing);
        let peer_id1 = get_common_peer_id(&correct_addresses_1());
        assert_eq!(handler0.peer_id(&NodeIndex(1)), peer_id1);
        assert_eq!(
            handler0.authentication_for(&NodeIndex(1)).encode(),
            handler1.authentication().encode()
        );
    }

    #[tokio::test]
    async fn non_validator_accepts_correct_authentication() {
        let crypto_basics = crypto_basics(NUM_NODES).await;
        let mut handler0 = Handler::new(
            None,
            crypto_basics.1.clone(),
            SessionId(43),
            correct_addresses_0(),
        )
        .await
        .unwrap();
        let handler1 = Handler::new(
            Some(crypto_basics.0[1].clone()),
            crypto_basics.1.clone(),
            SessionId(43),
            correct_addresses_1(),
        )
        .await
        .unwrap();
        assert!(handler0.handle_authentication(handler1.authentication().unwrap()));
        let missing_nodes = handler0.missing_nodes();
        let mut expected_missing: Vec<_> = (0..NUM_NODES).map(NodeIndex).collect();
        expected_missing.remove(1);
        assert_eq!(missing_nodes, expected_missing);
        let peer_id1 = get_common_peer_id(&correct_addresses_1());
        assert_eq!(handler0.peer_id(&NodeIndex(1)), peer_id1);
    }

    #[tokio::test]
    async fn ignores_badly_signed_authentication() {
        let crypto_basics = crypto_basics(NUM_NODES).await;
        let mut handler0 = Handler::new(
            Some(crypto_basics.0[0].clone()),
            crypto_basics.1.clone(),
            SessionId(43),
            correct_addresses_0(),
        )
        .await
        .unwrap();
        let handler1 = Handler::new(
            Some(crypto_basics.0[1].clone()),
            crypto_basics.1.clone(),
            SessionId(43),
            correct_addresses_1(),
        )
        .await
        .unwrap();
        let mut authentication = handler1.authentication().unwrap();
        authentication.1 = handler0.authentication().unwrap().1;
        assert!(!handler0.handle_authentication(authentication));
        let missing_nodes = handler0.missing_nodes();
        let expected_missing: Vec<_> = (1..NUM_NODES).map(NodeIndex).collect();
        assert_eq!(missing_nodes, expected_missing);
        assert!(handler0.authentication_for(&NodeIndex(1)).is_none());
    }

    #[tokio::test]
    async fn ignores_wrong_session_authentication() {
        let crypto_basics = crypto_basics(NUM_NODES).await;
        let mut handler0 = Handler::new(
            Some(crypto_basics.0[0].clone()),
            crypto_basics.1.clone(),
            SessionId(43),
            correct_addresses_0(),
        )
        .await
        .unwrap();
        let handler1 = Handler::new(
            Some(crypto_basics.0[1].clone()),
            crypto_basics.1.clone(),
            SessionId(44),
            correct_addresses_1(),
        )
        .await
        .unwrap();
        assert!(!handler0.handle_authentication(handler1.authentication().unwrap()));
        let missing_nodes = handler0.missing_nodes();
        let expected_missing: Vec<_> = (1..NUM_NODES).map(NodeIndex).collect();
        assert_eq!(missing_nodes, expected_missing);
        assert!(handler0.authentication_for(&NodeIndex(1)).is_none());
    }

    #[tokio::test]
    async fn ignores_own_authentication() {
        let awaited_crypto_basics = crypto_basics(NUM_NODES).await;
        let mut handler0 = Handler::new(
            Some(awaited_crypto_basics.0[0].clone()),
            awaited_crypto_basics.1.clone(),
            SessionId(43),
            correct_addresses_0(),
        )
        .await
        .unwrap();
        assert!(!handler0.handle_authentication(handler0.authentication().unwrap()));
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
            correct_addresses_0(),
        )
        .await
        .unwrap();
        let handler1 = Handler::new(
            Some(awaited_crypto_basics.0[1].clone()),
            awaited_crypto_basics.1.clone(),
            SessionId(43),
            correct_addresses_1(),
        )
        .await
        .unwrap();
        assert!(handler0.handle_authentication(handler1.authentication().unwrap()));
        let new_crypto_basics = crypto_basics(NUM_NODES).await;
        print!(
            "{:?}",
            handler0
                .update(
                    Some(new_crypto_basics.0[0].clone()),
                    new_crypto_basics.1.clone(),
                    correct_addresses_0()
                )
                .await
                .unwrap()
        );
        let missing_nodes = handler0.missing_nodes();
        let expected_missing: Vec<_> = (1..NUM_NODES).map(NodeIndex).collect();
        assert_eq!(missing_nodes, expected_missing);
        assert!(handler0.peer_id(&NodeIndex(1)).is_none());
        assert!(handler0.authentication_for(&NodeIndex(1)).is_none());
    }

    #[tokio::test]
    async fn uses_cached_authentication() {
        let awaited_crypto_basics = crypto_basics(NUM_NODES).await;
        let mut handler0 = Handler::new(
            Some(awaited_crypto_basics.0[0].clone()),
            awaited_crypto_basics.1.clone(),
            SessionId(43),
            correct_addresses_0(),
        )
        .await
        .unwrap();
        let mut handler1 = Handler::new(
            Some(awaited_crypto_basics.0[1].clone()),
            awaited_crypto_basics.1.clone(),
            SessionId(43),
            correct_addresses_1(),
        )
        .await
        .unwrap();
        assert!(handler0.handle_authentication(handler1.authentication().unwrap()));
        let new_crypto_basics = crypto_basics(NUM_NODES).await;
        assert!(handler1
            .update(
                Some(new_crypto_basics.0[1].clone()),
                new_crypto_basics.1.clone(),
                correct_addresses_1()
            )
            .await
            .unwrap()
            .is_empty());
        assert!(!handler0.handle_authentication(handler1.authentication().unwrap()));
        handler0
            .update(
                Some(new_crypto_basics.0[0].clone()),
                new_crypto_basics.1.clone(),
                correct_addresses_0(),
            )
            .await
            .unwrap();
        let missing_nodes = handler0.missing_nodes();
        let expected_missing: Vec<_> = (2..NUM_NODES).map(NodeIndex).collect();
        assert_eq!(missing_nodes, expected_missing);
        assert_eq!(
            handler0.peer_id(&NodeIndex(1)),
            get_common_peer_id(&correct_addresses_1())
        );
        assert_eq!(
            handler0.authentication_for(&NodeIndex(1)).encode(),
            handler1.authentication().encode()
        );
    }
}
