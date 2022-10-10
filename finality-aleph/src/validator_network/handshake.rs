use std::fmt::{Display, Error as FmtError, Formatter};

use aleph_primitives::AuthorityId;
use codec::{Decode, Encode};
use rand::Rng;
use tokio::time::{timeout, Duration};

use crate::{
    crypto::{verify, AuthorityPen, Signature},
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
    /// Signature error.
    SignatureError,
    /// Timeout.
    TimedOut,
}

impl Display for HandshakeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use HandshakeError::*;
        match self {
            SendError(e) => write!(f, "send error: {}", e),
            ReceiveError(e) => write!(f, "receive error: {}", e),
            SignatureError => write!(f, "signature error"),
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

/// Handshake challenge.
#[derive(Debug, Clone, Encode, Decode)]
struct Challenge {
    id: AuthorityId,
    nonce: [u8; 32],
}

impl Challenge {
    /// Prepare new challenge that contains ID of the creator.
    fn new(id: AuthorityId) -> Self {
        let nonce = rand::thread_rng().gen::<[u8; 32]>();
        Self { id, nonce }
    }
}

/// Handshake response.
#[derive(Debug, Clone, Encode, Decode)]
struct Response(Signature);

impl Response {
    /// Create a new response by signing the challenge.
    async fn new(pen: &AuthorityPen, challenge: &Challenge) -> Self {
        Self(pen.sign(&challenge.encode()).await)
    }

    /// Verify the Response sent by the peer.
    fn verify(&self, peer_id: &AuthorityId, challenge: &Challenge) -> bool {
        verify(peer_id, &challenge.encode(), &self.0)
    }
}

/// Performs the handshake. The goal is to obtain ID of the peer,
/// and split the communication stream into two halves.
pub async fn execute_v0_handshake<S: Splittable>(
    stream: S,
    authority_pen: AuthorityPen,
) -> Result<(S::Sender, S::Receiver, AuthorityId), HandshakeError> {
    // send challenge
    let our_challenge = Challenge::new(authority_pen.authority_id());
    let stream = send_data(stream, our_challenge.clone()).await?;
    // receive challenge
    let (stream, peer_challenge) = receive_data::<_, Challenge>(stream).await?;
    let peer_id = peer_challenge.id.clone();
    // send response
    let our_response = Response::new(&authority_pen, &peer_challenge).await;
    let stream = send_data(stream, our_response).await?;
    // receive response
    let (stream, peer_response) = receive_data::<_, Response>(stream).await?;
    // validate response
    if !peer_response.verify(&peer_id, &our_challenge) {
        return Err(HandshakeError::SignatureError);
    }
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
    use aleph_primitives::AuthorityId;
    use futures::{join, try_join};

    use super::{execute_v0_handshake, Challenge, HandshakeError, Response};
    use crate::{
        crypto::AuthorityPen,
        validator_network::{
            io::{receive_data, send_data},
            mock::{keys, MockSplittable},
            Splittable,
        },
    };

    pub async fn execute_malicious_v0_handshake<S: Splittable>(
        stream: S,
        authority_pen: AuthorityPen,
    ) -> Result<(S::Sender, S::Receiver, AuthorityId), HandshakeError> {
        // send challenge
        let our_challenge = Challenge::new(authority_pen.authority_id());
        let stream = send_data(stream, our_challenge.clone()).await?;
        // receive challenge
        let (stream, peer_challenge) = receive_data::<_, Challenge>(stream).await?;
        let peer_id = peer_challenge.id.clone();
        // WE PREPARE A FAKE RESPONSE WITH INVALID SIGNATURE
        // send fake response
        let fake_challenge = our_challenge.clone();
        let fake_response = Response::new(&authority_pen, &fake_challenge).await;
        let stream = send_data(stream, fake_response).await?;
        // receive response
        let (stream, peer_response) = receive_data::<_, Response>(stream).await?;
        // validate response
        if !peer_response.verify(&peer_id, &our_challenge) {
            return Err(HandshakeError::SignatureError);
        }
        let (sender, receiver) = stream.split();
        Ok((sender, receiver, peer_id))
    }

    pub async fn execute_broken_v0_handshake<S: Splittable>(
        stream: S,
        authority_pen: AuthorityPen,
        step: usize,
    ) -> Result<(), HandshakeError> {
        // WE CHANGE THE ORDER OF STEPS TO MAKE TESTS DETERMINISTIC
        // receive challenge
        let (stream, peer_challenge) = receive_data::<_, Challenge>(stream).await?;
        let _ = peer_challenge.id.clone();
        if step == 0 {
            return Ok(());
        }
        // send challenge
        let our_challenge = Challenge::new(authority_pen.authority_id());
        let stream = send_data(stream, our_challenge.clone()).await?;
        if step == 1 {
            return Ok(());
        }
        // receive response
        let (stream, _) = receive_data::<_, Response>(stream).await?;
        if step == 2 {
            return Ok(());
        }
        // send response
        let our_response = Response::new(&authority_pen, &peer_challenge).await;
        send_data(stream, our_response).await?;
        Ok(())
    }

    async fn break_v0_handshake_after(step: usize) -> HandshakeError {
        let (stream_a, stream_b) = MockSplittable::new(4096);
        let (id_a, pen_a) = keys().await;
        let (id_b, pen_b) = keys().await;
        assert_ne!(id_a, id_b);
        let (e, _) = join!(
            execute_v0_handshake(stream_a, pen_a),
            execute_broken_v0_handshake(stream_b, pen_b, step),
        );
        e.expect_err("should end with error")
    }

    fn assert_send_error(e: HandshakeError) {
        match e {
            HandshakeError::SendError(_) => (),
            x => panic!(
                "should end with HandshakeError::SendError, but we got {:?}",
                x
            ),
        };
    }

    fn assert_receive_error(e: HandshakeError) {
        match e {
            HandshakeError::ReceiveError(_) => (),
            x => panic!(
                "should end with HandshakeError::ReceiveError, but we got {:?}",
                x
            ),
        };
    }

    #[tokio::test]
    async fn handshake() {
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

    #[tokio::test]
    async fn broken_connection() {
        // break the connection even before the handshake starts
        let (stream_a, _) = MockSplittable::new(4096);
        let (_, pen_a) = keys().await;
        let e = execute_v0_handshake(stream_a, pen_a)
            .await
            .expect_err("should end with error");
        assert_send_error(e);
        // break the connection at precise step
        assert_receive_error(break_v0_handshake_after(0).await);
        assert_send_error(break_v0_handshake_after(1).await);
        assert_receive_error(break_v0_handshake_after(2).await);
    }

    #[tokio::test]
    async fn handshake_with_malicious_peer() {
        let (stream_a, stream_b) = MockSplittable::new(4096);
        let (id_a, pen_a) = keys().await;
        let (id_b, pen_b) = keys().await;
        assert_ne!(id_a, id_b);
        let (result_good, result_bad) = join!(
            execute_v0_handshake(stream_a, pen_a),
            execute_malicious_v0_handshake(stream_b, pen_b),
        );
        let (_, _, received_id_a) = result_bad.expect("bad handshake should work");
        assert_eq!(id_a, received_id_a);
        match result_good {
            Err(HandshakeError::SignatureError) => (),
            x => panic!(
                "should end with HandshakeError::SignatureError, but we got {:?}",
                x
            ),
        }
    }
}
