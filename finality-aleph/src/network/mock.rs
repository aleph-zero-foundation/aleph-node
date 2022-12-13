use std::{sync::Arc, time::Duration};

use aleph_primitives::KEY_TYPE;
use futures::{channel::mpsc, StreamExt};
use sp_keystore::{testing::KeyStore, CryptoStore};
use tokio::time::timeout;

use crate::{
    crypto::{AuthorityPen, AuthorityVerifier},
    AuthorityId, NodeIndex,
};

pub type MockData = Vec<u8>;

#[derive(Clone)]
pub struct Channel<T>(
    pub mpsc::UnboundedSender<T>,
    pub Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<T>>>,
);

const TIMEOUT_FAIL: Duration = Duration::from_secs(10);

impl<T> Channel<T> {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded();
        Channel(tx, Arc::new(tokio::sync::Mutex::new(rx)))
    }

    pub fn send(&self, msg: T) {
        self.0.unbounded_send(msg).unwrap();
    }

    pub async fn next(&mut self) -> Option<T> {
        timeout(TIMEOUT_FAIL, self.1.lock().await.next())
            .await
            .ok()
            .flatten()
    }

    pub async fn take(&mut self, n: usize) -> Vec<T> {
        timeout(
            TIMEOUT_FAIL,
            self.1.lock().await.by_ref().take(n).collect::<Vec<_>>(),
        )
        .await
        .unwrap_or_default()
    }

    pub async fn try_next(&self) -> Option<T> {
        self.1.lock().await.try_next().unwrap_or(None)
    }

    pub async fn close(self) -> Option<T> {
        self.0.close_channel();
        self.try_next().await
    }
}

impl<T> Default for Channel<T> {
    fn default() -> Self {
        Self::new()
    }
}

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
