use std::fmt::{Display, Error as FmtError, Formatter};

use aleph_primitives::AuthorityId;
use tokio::time::{timeout, Duration};

use crate::{
    crypto::AuthorityPen,
    validator_network::{
        io::{receive_data, send_data, ReceiveError, SendError},
        Splittable,
    },
};

pub const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

/// Handshake error.
#[derive(Debug)]
pub enum HandshakeError {
    /// Send error.
    SendError(SendError),
    /// Receive error.
    ReceiveError(ReceiveError),
    /// Timeout.
    TimedOut,
}

impl Display for HandshakeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use HandshakeError::*;
        match self {
            SendError(e) => write!(f, "send error: {}", e),
            ReceiveError(e) => write!(f, "receive error: {}", e),
            TimedOut => write!(f, "timed out"),
        }
    }
}

impl From<SendError> for HandshakeError {
    fn from(e: SendError) -> Self {
        HandshakeError::SendError(e)
    }
}

impl From<ReceiveError> for HandshakeError {
    fn from(e: ReceiveError) -> Self {
        HandshakeError::ReceiveError(e)
    }
}

/// Performs the handshake. The goal is to obtain ID of the peer,
/// and split the communication stream into two halves.
/// Current version makes an unrealistic assumption that the peer is not malicious.
/// To be rewritten.
pub async fn execute_v0_handshake<S: Splittable>(
    stream: S,
    authority_pen: AuthorityPen,
) -> Result<(S::Sender, S::Receiver, AuthorityId), HandshakeError> {
    let authority_id = authority_pen.authority_id();
    let (stream, peer_id) = receive_data(send_data(stream, authority_id).await?).await?;
    let (sender, receiver) = stream.split();
    Ok((sender, receiver, peer_id))
}

/// Wrapper that adds timeout to the function performing handshake.
pub async fn v0_handshake<S: Splittable>(
    stream: S,
    authority_pen: AuthorityPen,
) -> Result<(S::Sender, S::Receiver, AuthorityId), HandshakeError> {
    timeout(
        HANDSHAKE_TIMEOUT,
        execute_v0_handshake(stream, authority_pen),
    )
    .await
    .map_err(|_| HandshakeError::TimedOut)?
}

#[cfg(test)]
mod tests {
    use futures::try_join;

    use super::execute_v0_handshake;
    use crate::validator_network::mock::{keys, MockSplittable};

    // Only one basic test for now, as the handshake will be rewritten.

    #[tokio::test]
    async fn dual_handshake() {
        let (stream_a, stream_b) = MockSplittable::new(4096);
        let (id_a, pen_a) = keys().await;
        let (id_b, pen_b) = keys().await;
        assert_ne!(id_a, id_b);
        let ((_, _, received_id_b), (_, _, received_id_a)) = try_join!(
            execute_v0_handshake(stream_a, pen_a),
            execute_v0_handshake(stream_b, pen_b),
        )
        .expect("handshake should work");
        assert_eq!(id_a, received_id_a);
        assert_eq!(id_b, received_id_b);
    }
}
