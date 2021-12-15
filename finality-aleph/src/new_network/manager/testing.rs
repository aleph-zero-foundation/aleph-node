use crate::{
    crypto::{AuthorityPen, AuthorityVerifier},
    new_network::{NetworkIdentity, PeerId},
    AuthorityId, NodeIndex,
};
use aleph_primitives::KEY_TYPE;
use sc_network::{multiaddr::Protocol as ScProtocol, Multiaddr as ScMultiaddr, PeerId as ScPeerId};
use sp_keystore::{testing::KeyStore, CryptoStore};
use std::{net::Ipv4Addr, sync::Arc};

pub async fn crypto_basics(
    num_crypto_basics: usize,
) -> (Vec<(NodeIndex, AuthorityPen)>, AuthorityVerifier) {
    let keystore = Arc::new(KeyStore::new());
    let mut auth_ids = Vec::with_capacity(num_crypto_basics);
    for _ in 0..num_crypto_basics {
        let pk = keystore.ed25519_generate_new(KEY_TYPE, None).await.unwrap();
        auth_ids.push(AuthorityId::from(pk));
    }
    let mut result = Vec::with_capacity(num_crypto_basics);
    for (i, auth_id) in auth_ids.iter().enumerate() {
        result.push((
            NodeIndex(i),
            AuthorityPen::new(auth_id.clone(), keystore.clone())
                .await
                .expect("The keys should sign successfully"),
        ));
    }
    (result, AuthorityVerifier::new(auth_ids))
}

pub fn address(text: &str) -> ScMultiaddr {
    text.parse().unwrap()
}

fn random_address(peer_id: PeerId) -> ScMultiaddr {
    ScMultiaddr::empty()
        .with(ScProtocol::Ip4(Ipv4Addr::new(
            192,
            168,
            rand::random(),
            rand::random(),
        )))
        .with(ScProtocol::Tcp(30333))
        .with(ScProtocol::P2p(peer_id.0.into()))
}

pub struct MockNetworkIdentity {
    addresses: Vec<ScMultiaddr>,
    peer_id: PeerId,
}

impl MockNetworkIdentity {
    pub fn new() -> Self {
        let peer_id = ScPeerId::random().into();
        let addresses = (0..3).map(|_| random_address(peer_id)).collect();
        MockNetworkIdentity { addresses, peer_id }
    }
}

impl NetworkIdentity for MockNetworkIdentity {
    fn identity(&self) -> (Vec<ScMultiaddr>, PeerId) {
        (self.addresses.clone(), self.peer_id)
    }
}
