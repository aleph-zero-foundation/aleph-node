use aleph_primitives::AuthorityId;
use futures::future::pending;
use sp_core::crypto::KeyTypeId;
use sp_keystore::{testing::KeyStore, CryptoStore};

/// Network represents an interface for opening and closing connections with other Validators,
/// and sending direct messages between them.
///
/// Note on Network reliability and security: it is neither assumed that the sent messages must be
/// always delivered, nor the established connections must be secure in any way. The Network
/// implementation might fail to deliver any specific message, so messages have to be resend while
/// they still should be delivered.
#[async_trait::async_trait]
pub trait Network<A, D>: Send {
    /// Add the peer to the set of connected peers.
    fn add_connection(&mut self, peer: AuthorityId, addresses: Vec<A>);

    /// Remove the peer from the set of connected peers and close the connection.
    fn remove_connection(&mut self, peer: AuthorityId);

    /// Send a message to a single peer.
    /// This function should be implemented in a non-blocking manner.
    fn send(&self, data: D, recipient: AuthorityId);

    /// Receive a message from the network.
    async fn next(&mut self) -> Option<D>;

    /// Return the public key associated with this Network.
    fn public_key(&self) -> AuthorityId;

    /// Return the list of addresses that are externally accessible.
    fn addresses(&self) -> Vec<A>;
}

/// Remove after we have a proper implementation.
pub struct MockNetwork(AuthorityId);

impl MockNetwork {
    #[allow(dead_code)]
    pub async fn new() -> MockNetwork {
        let key_type: KeyTypeId = KeyTypeId(*b"alp0");
        let key_store = KeyStore::new();
        let pk = key_store
            .ed25519_generate_new(key_type, Some("MockNetwork"))
            .await
            .unwrap();
        let authority_id = AuthorityId::from(pk);
        MockNetwork(authority_id)
    }
}

#[async_trait::async_trait]
impl<D: Send> Network<String, D> for MockNetwork {
    fn add_connection(&mut self, _peer: AuthorityId, _addresses: Vec<String>) {}

    fn remove_connection(&mut self, _peer: AuthorityId) {}

    fn send(&self, _data: D, _recipient: AuthorityId) {}

    async fn next(&mut self) -> Option<D> {
        // MockNetwork never receives any messages
        Some(pending::<D>().await)
    }

    fn public_key(&self) -> AuthorityId {
        self.0.clone()
    }

    fn addresses(&self) -> Vec<String> {
        vec![]
    }
}
