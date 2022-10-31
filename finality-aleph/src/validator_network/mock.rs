use std::sync::Arc;
#[cfg(test)]
use std::{
    collections::HashMap,
    io::Result as IoResult,
    pin::Pin,
    task::{Context, Poll},
};

use aleph_primitives::{AuthorityId, KEY_TYPE};
use sp_keystore::{testing::KeyStore, CryptoStore};
use tokio::io::{duplex, AsyncRead, AsyncWrite, DuplexStream, ReadBuf};

use crate::{crypto::AuthorityPen, validator_network::Splittable};

/// Create a random authority id and pen pair.
pub async fn key() -> (AuthorityId, AuthorityPen) {
    let keystore = Arc::new(KeyStore::new());
    let id: AuthorityId = keystore
        .ed25519_generate_new(KEY_TYPE, None)
        .await
        .unwrap()
        .into();
    let pen = AuthorityPen::new(id.clone(), keystore)
        .await
        .expect("keys shoud sign successfully");
    (id, pen)
}

/// Create a HashMap with authority ids as keys and pens as values.
pub async fn random_keys(n_peers: usize) -> HashMap<AuthorityId, AuthorityPen> {
    let mut result = HashMap::with_capacity(n_peers);
    for _ in 0..n_peers {
        let (id, pen) = key().await;
        result.insert(id, pen);
    }
    assert_eq!(result.len(), n_peers);
    result
}

/// A mock that can be split into two streams.
pub struct MockSplittable {
    incoming_data: DuplexStream,
    outgoing_data: DuplexStream,
}

impl MockSplittable {
    /// Create a pair of mock splittables connected to each other.
    pub fn new(max_buf_size: usize) -> (Self, Self) {
        let (in_a, out_b) = duplex(max_buf_size);
        let (in_b, out_a) = duplex(max_buf_size);
        (
            MockSplittable {
                incoming_data: in_a,
                outgoing_data: out_a,
            },
            MockSplittable {
                incoming_data: in_b,
                outgoing_data: out_b,
            },
        )
    }
}

impl AsyncRead for MockSplittable {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<IoResult<()>> {
        Pin::new(&mut self.get_mut().incoming_data).poll_read(cx, buf)
    }
}

impl AsyncWrite for MockSplittable {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<IoResult<usize>> {
        Pin::new(&mut self.get_mut().outgoing_data).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Pin::new(&mut self.get_mut().outgoing_data).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Pin::new(&mut self.get_mut().outgoing_data).poll_shutdown(cx)
    }
}

impl Splittable for MockSplittable {
    type Sender = DuplexStream;
    type Receiver = DuplexStream;

    fn split(self) -> (Self::Sender, Self::Receiver) {
        (self.outgoing_data, self.incoming_data)
    }
}
