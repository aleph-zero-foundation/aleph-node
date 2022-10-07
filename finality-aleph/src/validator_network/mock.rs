use std::sync::Arc;
#[cfg(test)]
use std::{
    io::Result as IoResult,
    pin::Pin,
    task::{Context, Poll},
};

use aleph_primitives::{AuthorityId, KEY_TYPE};
use sp_keystore::{testing::KeyStore, CryptoStore};
use tokio::io::{duplex, AsyncRead, AsyncWrite, DuplexStream, ReadBuf};

use crate::{crypto::AuthorityPen, validator_network::Splittable};

/// Create a single authority id and pen of the same type, not related to each other.
pub async fn keys() -> (AuthorityId, AuthorityPen) {
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
