use std::{sync::Arc, time::Duration};

use aleph_primitives::KEY_TYPE;
use codec::{Decode, Encode, Output};
use futures::{channel::mpsc, StreamExt};
use sp_keystore::{testing::KeyStore, CryptoStore};
use tokio::time::timeout;

use crate::{
    crypto::{AuthorityPen, AuthorityVerifier},
    AuthorityId, NodeIndex,
};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct MockData {
    data: u32,
    filler: Vec<u8>,
    decodes: bool,
}

impl MockData {
    pub fn new(data: u32, filler_size: usize) -> MockData {
        MockData {
            data,
            filler: vec![0; filler_size],
            decodes: true,
        }
    }
}

impl Encode for MockData {
    fn size_hint(&self) -> usize {
        self.data.size_hint() + self.filler.size_hint() + self.decodes.size_hint()
    }

    fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
        // currently this is exactly the default behaviour, but we still
        // need it here to make sure that decode works in the future
        self.data.encode_to(dest);
        self.filler.encode_to(dest);
        self.decodes.encode_to(dest);
    }
}

impl Decode for MockData {
    fn decode<I: codec::Input>(value: &mut I) -> Result<Self, codec::Error> {
        let data = u32::decode(value)?;
        let filler = Vec::<u8>::decode(value)?;
        let decodes = bool::decode(value)?;
        if !decodes {
            return Err("Simulated decode failure.".into());
        }
        Ok(Self {
            data,
            filler,
            decodes,
        })
    }
}

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
