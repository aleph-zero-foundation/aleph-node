use std::collections::HashMap;

use aleph_bft::NodeCount;
use codec::Encode;

use crate::{
    crypto::{AuthorityPen, AuthorityVerifier},
    network::{
        manager::{AuthData, Authentication},
        Multiaddress, PeerId,
    },
    NodeIndex, SessionId,
};

#[derive(Debug)]
pub enum SessionInfo<M: Multiaddress> {
    SessionId(SessionId),
    OwnAuthentication(Authentication<M>),
}

impl<M: Multiaddress> SessionInfo<M> {
    fn session_id(&self) -> SessionId {
        match self {
            SessionInfo::SessionId(session_id) => *session_id,
            SessionInfo::OwnAuthentication((auth_data, _)) => auth_data.session_id,
        }
    }
}

type PeerAuthentications<M> = (Authentication<M>, Option<Authentication<M>>);

/// A struct for handling authentications for a given session and maintaining
/// mappings between PeerIds and NodeIndexes within that session.
pub struct Handler<M: Multiaddress> {
    peers_by_node: HashMap<NodeIndex, M::PeerId>,
    authentications: HashMap<M::PeerId, PeerAuthentications<M>>,
    session_info: SessionInfo<M>,
    own_peer_id: M::PeerId,
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

enum CommonPeerId<PID: PeerId> {
    Unknown,
    Unique(PID),
    NotUnique,
}

impl<PID: PeerId> From<CommonPeerId<PID>> for Option<PID> {
    fn from(cpi: CommonPeerId<PID>) -> Self {
        use CommonPeerId::*;
        match cpi {
            Unique(peer_id) => Some(peer_id),
            Unknown | NotUnique => None,
        }
    }
}

impl<PID: PeerId> CommonPeerId<PID> {
    fn aggregate(self, peer_id: PID) -> Self {
        use CommonPeerId::*;
        match self {
            Unknown => Unique(peer_id),
            Unique(current_peer_id) => match peer_id == current_peer_id {
                true => Unique(current_peer_id),
                false => NotUnique,
            },
            NotUnique => NotUnique,
        }
    }
}

fn get_common_peer_id<M: Multiaddress>(addresses: &[M]) -> Option<M::PeerId> {
    addresses
        .iter()
        .fold(
            CommonPeerId::Unknown,
            |common_peer_id, address| match address.get_peer_id() {
                Some(peer_id) => common_peer_id.aggregate(peer_id),
                None => CommonPeerId::NotUnique,
            },
        )
        .into()
}

fn retrieve_peer_id<M: Multiaddress>(addresses: &[M]) -> Result<M::PeerId, HandlerError> {
    if addresses.is_empty() {
        return Err(HandlerError::NoP2pAddresses);
    }
    get_common_peer_id(addresses).ok_or(HandlerError::MultiplePeerIds)
}

async fn construct_session_info<M: Multiaddress>(
    authority_index_and_pen: &Option<(NodeIndex, AuthorityPen)>,
    session_id: SessionId,
    addresses: Vec<M>,
) -> Result<(SessionInfo<M>, M::PeerId), HandlerError> {
    let addresses: Vec<_> = addresses
        .into_iter()
        .filter(|address| address.get_peer_id().is_some())
        .collect();
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

impl<M: Multiaddress> Handler<M> {
    /// Returns an error if the set of addresses contains no external libp2p addresses, or contains
    /// at least two such addresses with differing PeerIds.
    pub async fn new(
        authority_index_and_pen: Option<(NodeIndex, AuthorityPen)>,
        authority_verifier: AuthorityVerifier,
        session_id: SessionId,
        addresses: Vec<M>,
    ) -> Result<Handler<M>, HandlerError> {
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
    pub fn authentication(&self) -> Option<Authentication<M>> {
        match &self.session_info {
            SessionInfo::SessionId(_) => None,
            SessionInfo::OwnAuthentication(own_authentication) => Some(own_authentication.clone()),
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

    /// Verifies the authentication, uses it to update mappings, and returns whether we should
    /// remain connected to the multiaddresses.
    pub fn handle_authentication(&mut self, authentication: Authentication<M>) -> bool {
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
    pub fn peer_id(&self, node_id: &NodeIndex) -> Option<M::PeerId> {
        self.peers_by_node.get(node_id).copied()
    }

    /// Returns maping from NodeIndex to PeerId
    pub fn peers(&self) -> HashMap<NodeIndex, M::PeerId> {
        self.peers_by_node.clone()
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
        addresses: Vec<M>,
    ) -> Result<Vec<M>, HandlerError> {
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
            self.handle_authentication(auth);
            if let Some(auth) = maybe_auth {
                self.handle_authentication(auth);
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
        network::{
            mock::{crypto_basics, MockMultiaddress, MockNetworkIdentity, MockPeerId},
            NetworkIdentity,
        },
        NodeIndex, SessionId,
    };

    const NUM_NODES: usize = 7;

    #[tokio::test]
    async fn creates_with_correct_data() {
        let mut crypto_basics = crypto_basics(NUM_NODES).await;
        assert!(Handler::new(
            Some(crypto_basics.0.pop().unwrap()),
            crypto_basics.1,
            SessionId(43),
            MockNetworkIdentity::new().identity().0,
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
            MockNetworkIdentity::new().identity().0,
        )
        .await
        .is_ok());
    }

    #[tokio::test]
    async fn creates_without_node_index_nor_authority_pen() {
        let crypto_basics = crypto_basics(NUM_NODES).await;
        assert!(Handler::new(
            None,
            crypto_basics.1,
            SessionId(43),
            MockNetworkIdentity::new().identity().0,
        )
        .await
        .is_ok());
    }

    #[tokio::test]
    async fn identifies_whether_node_is_authority_in_current_session() {
        let mut crypto_basics = crypto_basics(NUM_NODES).await;
        let no_authority_handler = Handler::new(
            None,
            crypto_basics.1.clone(),
            SessionId(43),
            MockNetworkIdentity::new().identity().0,
        )
        .await
        .unwrap();
        let authority_handler = Handler::new(
            Some(crypto_basics.0.pop().unwrap()),
            crypto_basics.1,
            SessionId(43),
            MockNetworkIdentity::new().identity().0,
        )
        .await
        .unwrap();
        assert!(!no_authority_handler.is_validator());
        assert!(authority_handler.is_validator());
    }

    #[tokio::test]
    async fn non_validator_handler_returns_none_for_authentication() {
        let crypto_basics = crypto_basics(NUM_NODES).await;
        assert!(Handler::new(
            None,
            crypto_basics.1,
            SessionId(43),
            MockNetworkIdentity::new().identity().0
        )
        .await
        .unwrap()
        .authentication()
        .is_none());
    }

    #[tokio::test]
    async fn fails_to_create_with_no_addresses() {
        let mut crypto_basics = crypto_basics(NUM_NODES).await;
        assert!(matches!(
            Handler::new(
                Some(crypto_basics.0.pop().unwrap()),
                crypto_basics.1,
                SessionId(43),
                Vec::<MockMultiaddress>::new()
            )
            .await,
            Err(HandlerError::NoP2pAddresses)
        ));
    }

    #[tokio::test]
    async fn fails_to_create_with_non_unique_peer_id() {
        let mut crypto_basics = crypto_basics(NUM_NODES).await;
        let addresses = vec![
            MockMultiaddress::random_with_id(MockPeerId::random()),
            MockMultiaddress::random_with_id(MockPeerId::random()),
        ];
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
        let addresses = MockNetworkIdentity::new().identity().0;
        let mut handler0 = Handler::new(
            Some(crypto_basics.0.pop().unwrap()),
            crypto_basics.1.clone(),
            SessionId(43),
            addresses.clone(),
        )
        .await
        .unwrap();
        assert!(matches!(
            handler0
                .update(None, crypto_basics.1.clone(), addresses)
                .await,
            Err(HandlerError::TypeChange)
        ));
    }

    #[tokio::test]
    async fn fails_to_update_from_non_validator_to_validator() {
        let mut crypto_basics = crypto_basics(NUM_NODES).await;
        let addresses = MockNetworkIdentity::new().identity().0;
        let mut handler0 = Handler::new(
            None,
            crypto_basics.1.clone(),
            SessionId(43),
            addresses.clone(),
        )
        .await
        .unwrap();
        assert!(matches!(
            handler0
                .update(
                    Some(crypto_basics.0.pop().unwrap()),
                    crypto_basics.1.clone(),
                    addresses,
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
            MockNetworkIdentity::new().identity().0,
        )
        .await
        .unwrap();
        assert!(handler0.peer_id(&NodeIndex(0)).is_none());
    }

    #[tokio::test]
    async fn misses_all_other_nodes_initially() {
        let mut crypto_basics = crypto_basics(NUM_NODES).await;
        let handler0 = Handler::new(
            Some(crypto_basics.0.pop().unwrap()),
            crypto_basics.1,
            SessionId(43),
            MockNetworkIdentity::new().identity().0,
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
            MockNetworkIdentity::new().identity().0,
        )
        .await
        .unwrap();
        let addresses = MockNetworkIdentity::new().identity().0;
        let handler1 = Handler::new(
            Some(crypto_basics.0[1].clone()),
            crypto_basics.1.clone(),
            SessionId(43),
            addresses.clone(),
        )
        .await
        .unwrap();
        assert!(handler0.handle_authentication(handler1.authentication().unwrap()));
        let missing_nodes = handler0.missing_nodes();
        let expected_missing: Vec<_> = (2..NUM_NODES).map(NodeIndex).collect();
        assert_eq!(missing_nodes, expected_missing);
        let peer_id1 = get_common_peer_id(&addresses);
        assert_eq!(handler0.peer_id(&NodeIndex(1)), peer_id1);
    }

    #[tokio::test]
    async fn non_validator_accepts_correct_authentication() {
        let crypto_basics = crypto_basics(NUM_NODES).await;
        let mut handler0 = Handler::new(
            None,
            crypto_basics.1.clone(),
            SessionId(43),
            MockNetworkIdentity::new().identity().0,
        )
        .await
        .unwrap();
        let addresses = MockNetworkIdentity::new().identity().0;
        let handler1 = Handler::new(
            Some(crypto_basics.0[1].clone()),
            crypto_basics.1.clone(),
            SessionId(43),
            addresses.clone(),
        )
        .await
        .unwrap();
        assert!(handler0.handle_authentication(handler1.authentication().unwrap()));
        let missing_nodes = handler0.missing_nodes();
        let mut expected_missing: Vec<_> = (0..NUM_NODES).map(NodeIndex).collect();
        expected_missing.remove(1);
        assert_eq!(missing_nodes, expected_missing);
        let peer_id1 = get_common_peer_id(&addresses);
        assert_eq!(handler0.peer_id(&NodeIndex(1)), peer_id1);
    }

    #[tokio::test]
    async fn ignores_badly_signed_authentication() {
        let crypto_basics = crypto_basics(NUM_NODES).await;
        let mut handler0 = Handler::new(
            Some(crypto_basics.0[0].clone()),
            crypto_basics.1.clone(),
            SessionId(43),
            MockNetworkIdentity::new().identity().0,
        )
        .await
        .unwrap();
        let handler1 = Handler::new(
            Some(crypto_basics.0[1].clone()),
            crypto_basics.1.clone(),
            SessionId(43),
            MockNetworkIdentity::new().identity().0,
        )
        .await
        .unwrap();
        let mut authentication = handler1.authentication().unwrap();
        authentication.1 = handler0.authentication().unwrap().1;
        assert!(!handler0.handle_authentication(authentication));
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
            MockNetworkIdentity::new().identity().0,
        )
        .await
        .unwrap();
        let handler1 = Handler::new(
            Some(crypto_basics.0[1].clone()),
            crypto_basics.1.clone(),
            SessionId(44),
            MockNetworkIdentity::new().identity().0,
        )
        .await
        .unwrap();
        assert!(!handler0.handle_authentication(handler1.authentication().unwrap()));
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
            MockNetworkIdentity::new().identity().0,
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
            MockNetworkIdentity::new().identity().0,
        )
        .await
        .unwrap();
        let handler1 = Handler::new(
            Some(awaited_crypto_basics.0[1].clone()),
            awaited_crypto_basics.1.clone(),
            SessionId(43),
            MockNetworkIdentity::new().identity().0,
        )
        .await
        .unwrap();
        assert!(handler0.handle_authentication(handler1.authentication().unwrap()));
        let new_crypto_basics = crypto_basics(NUM_NODES).await;
        handler0
            .update(
                Some(new_crypto_basics.0[0].clone()),
                new_crypto_basics.1.clone(),
                MockNetworkIdentity::new().identity().0,
            )
            .await
            .unwrap();
        let missing_nodes = handler0.missing_nodes();
        let expected_missing: Vec<_> = (1..NUM_NODES).map(NodeIndex).collect();
        assert_eq!(missing_nodes, expected_missing);
        assert!(handler0.peer_id(&NodeIndex(1)).is_none());
    }

    #[tokio::test]
    async fn uses_cached_authentication() {
        let awaited_crypto_basics = crypto_basics(NUM_NODES).await;
        let addresses0 = MockNetworkIdentity::new().identity().0;
        let mut handler0 = Handler::new(
            Some(awaited_crypto_basics.0[0].clone()),
            awaited_crypto_basics.1.clone(),
            SessionId(43),
            addresses0.clone(),
        )
        .await
        .unwrap();
        let addresses1 = MockNetworkIdentity::new().identity().0;
        let mut handler1 = Handler::new(
            Some(awaited_crypto_basics.0[1].clone()),
            awaited_crypto_basics.1.clone(),
            SessionId(43),
            addresses1.clone(),
        )
        .await
        .unwrap();
        assert!(handler0.handle_authentication(handler1.authentication().unwrap()));
        let new_crypto_basics = crypto_basics(NUM_NODES).await;
        assert!(handler1
            .update(
                Some(new_crypto_basics.0[1].clone()),
                new_crypto_basics.1.clone(),
                addresses1.clone(),
            )
            .await
            .unwrap()
            .is_empty());
        assert!(!handler0.handle_authentication(handler1.authentication().unwrap()));
        handler0
            .update(
                Some(new_crypto_basics.0[0].clone()),
                new_crypto_basics.1.clone(),
                addresses0,
            )
            .await
            .unwrap();
        let missing_nodes = handler0.missing_nodes();
        let expected_missing: Vec<_> = (2..NUM_NODES).map(NodeIndex).collect();
        assert_eq!(missing_nodes, expected_missing);
        assert_eq!(
            handler0.peer_id(&NodeIndex(1)),
            get_common_peer_id(&addresses1)
        );
    }
}
