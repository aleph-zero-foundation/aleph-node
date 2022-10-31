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
    /// Challenge contains invalid peer id.
    ChallengeError(AuthorityId, AuthorityId),
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
            ChallengeError(expected, got) => write!(
                f,
                "challenge error, expected peer {}, received from {}",
                expected, got
            ),
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

/// Handshake challenge. Contains public key of the creator, and a random nonce.
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

/// Handshake response. Contains public key of the creator, and signature
/// related to the received challenge.
#[derive(Debug, Clone, Encode, Decode)]
struct Response {
    id: AuthorityId,
    signature: Signature,
}

impl Response {
    /// Create a new response by signing the challenge.
    async fn new(pen: &AuthorityPen, challenge: &Challenge) -> Self {
        Self {
            id: pen.authority_id(),
            signature: pen.sign(&challenge.encode()).await,
        }
    }

    /// Verify the Response sent by the peer.
    fn verify(&self, challenge: &Challenge) -> bool {
        verify(&self.id, &challenge.encode(), &self.signature)
    }
}

/// Performs the handshake with a peer that called us.
/// The goal is to obtain the public key of the peer, and split
/// the communication stream into two halves.
/// The peer needs to prove their identity by signing a randomly generated
/// challenge, but apart from that, the returned communication channels
/// will NOT be secured in any way. We assume that if the channel is
/// compromised after the handshake, the peer will establish another connection,
/// which will replace the current one.
pub async fn execute_v0_handshake_incoming<S: Splittable>(
    stream: S,
    authority_pen: AuthorityPen,
) -> Result<(S::Sender, S::Receiver, AuthorityId), HandshakeError> {
    // send challenge
    let our_challenge = Challenge::new(authority_pen.authority_id());
    let stream = send_data(stream, our_challenge.clone()).await?;
    // receive response
    let (stream, peer_response) = receive_data::<_, Response>(stream).await?;
    // validate response
    if !peer_response.verify(&our_challenge) {
        return Err(HandshakeError::SignatureError);
    }
    let (sender, receiver) = stream.split();
    let peer_id = peer_response.id;
    Ok((sender, receiver, peer_id))
}

/// Performs the handshake with a peer that we called. We assume that their
/// public key is known to us.
/// The goal is to authenticate ourselves, and split the communication stream
/// into two halves.
/// We need to prove our identity by signing a randomly generated
/// challenge, but apart from that, the returned communication channels
/// will NOT be secured in any way. We assume that if the channel is
/// compromised after the handshake, we will establish another connection,
/// which will replace the current one.
pub async fn execute_v0_handshake_outgoing<S: Splittable>(
    stream: S,
    authority_pen: AuthorityPen,
    peer_id: AuthorityId,
) -> Result<(S::Sender, S::Receiver), HandshakeError> {
    // receive challenge
    let (stream, peer_challenge) = receive_data::<_, Challenge>(stream).await?;
    if peer_id != peer_challenge.id {
        return Err(HandshakeError::ChallengeError(peer_id, peer_challenge.id));
    }
    // send response
    let our_response = Response::new(&authority_pen, &peer_challenge).await;
    let stream = send_data(stream, our_response).await?;
    let (sender, receiver) = stream.split();
    Ok((sender, receiver))
}

/// Wrapper that adds timeout to the function performing handshake.
pub async fn v0_handshake_incoming<S: Splittable>(
    stream: S,
    authority_pen: AuthorityPen,
) -> Result<(S::Sender, S::Receiver, AuthorityId), HandshakeError> {
    timeout(
        HANDSHAKE_TIMEOUT,
        execute_v0_handshake_incoming(stream, authority_pen),
    )
    .await
    .map_err(|_| HandshakeError::TimedOut)?
}

/// Wrapper that adds timeout to the function performing handshake.
pub async fn v0_handshake_outgoing<S: Splittable>(
    stream: S,
    authority_pen: AuthorityPen,
    peer_id: AuthorityId,
) -> Result<(S::Sender, S::Receiver), HandshakeError> {
    timeout(
        HANDSHAKE_TIMEOUT,
        execute_v0_handshake_outgoing(stream, authority_pen, peer_id),
    )
    .await
    .map_err(|_| HandshakeError::TimedOut)?
}

#[cfg(test)]
mod tests {
    use futures::{join, try_join};

    use super::{
        execute_v0_handshake_incoming, execute_v0_handshake_outgoing, Challenge, HandshakeError,
        Response,
    };
    use crate::{
        crypto::AuthorityPen,
        validator_network::{
            io::{receive_data, send_data},
            mock::{key, MockSplittable},
            Splittable,
        },
    };

    fn assert_send_error<T: std::fmt::Debug>(result: Result<T, HandshakeError>) {
        match result {
            Err(HandshakeError::SendError(_)) => (),
            x => panic!(
                "should end with HandshakeError::SendError, but we got {:?}",
                x
            ),
        };
    }

    fn assert_receive_error<T: std::fmt::Debug>(result: Result<T, HandshakeError>) {
        match result {
            Err(HandshakeError::ReceiveError(_)) => (),
            x => panic!(
                "should end with HandshakeError::ReceiveError, but we got {:?}",
                x
            ),
        };
    }

    fn assert_signature_error<T: std::fmt::Debug>(result: Result<T, HandshakeError>) {
        match result {
            Err(HandshakeError::SignatureError) => (),
            x => panic!(
                "should end with HandshakeError::SignatureError, but we got {:?}",
                x
            ),
        };
    }

    fn assert_challenge_error<T: std::fmt::Debug>(result: Result<T, HandshakeError>) {
        match result {
            Err(HandshakeError::ChallengeError(_, _)) => (),
            x => panic!(
                "should end with HandshakeError::ChallengeError, but we got {:?}",
                x
            ),
        };
    }

    #[tokio::test]
    async fn handshake() {
        let (stream_a, stream_b) = MockSplittable::new(4096);
        let (id_a, pen_a) = key().await;
        let (id_b, pen_b) = key().await;
        assert_ne!(id_a, id_b);
        let ((_, _, received_id_b), (_, _)) = try_join!(
            execute_v0_handshake_incoming(stream_a, pen_a),
            execute_v0_handshake_outgoing(stream_b, pen_b, id_a),
        )
        .expect("handshake should work");
        assert_eq!(id_b, received_id_b);
    }

    #[tokio::test]
    async fn handshake_with_malicious_server_peer() {
        async fn execute_malicious_v0_handshake_incoming<S: Splittable>(stream: S) {
            let (fake_id, _) = key().await;
            // send challenge with incorrect id
            let our_challenge = Challenge::new(fake_id);
            send_data(stream, our_challenge.clone())
                .await
                .expect("should send");
            // wait forever
            futures::future::pending::<()>().await;
        }

        let (stream_a, stream_b) = MockSplittable::new(4096);
        let (id_a, _) = key().await;
        let (_, pen_b) = key().await;
        tokio::select! {
            _ = execute_malicious_v0_handshake_incoming(stream_a) => panic!("should wait"),
            result = execute_v0_handshake_outgoing(stream_b, pen_b, id_a) => assert_challenge_error(result),
        }
    }

    #[tokio::test]
    async fn handshake_with_malicious_client_peer_fake_challenge() {
        pub async fn execute_malicious_v0_handshake_outgoing_fake_challenge<S: Splittable>(
            stream: S,
            authority_pen: AuthorityPen,
        ) {
            // receive challenge
            let (stream, _) = receive_data::<_, Challenge>(stream)
                .await
                .expect("should receive");
            // prepare fake challenge
            let (fake_id, _) = key().await;
            let fake_challenge = Challenge::new(fake_id);
            // send response with substituted challenge
            let our_response = Response::new(&authority_pen, &fake_challenge).await;
            send_data(stream, our_response).await.expect("should send");
            futures::future::pending::<()>().await;
        }

        let (stream_a, stream_b) = MockSplittable::new(4096);
        let (_, pen_a) = key().await;
        let (_, pen_b) = key().await;
        tokio::select! {
            result = execute_v0_handshake_incoming(stream_a, pen_a) => assert_signature_error(result),
            _ = execute_malicious_v0_handshake_outgoing_fake_challenge(stream_b, pen_b) => panic!("should wait"),
        }
    }

    #[tokio::test]
    async fn handshake_with_malicious_client_peer_fake_signature() {
        pub async fn execute_malicious_v0_handshake_outgoing_fake_signature<S: Splittable>(
            stream: S,
            authority_pen: AuthorityPen,
        ) {
            // receive challenge
            let (stream, challenge) = receive_data::<_, Challenge>(stream)
                .await
                .expect("should receive");
            // prepare fake id
            let (fake_id, _) = key().await;
            // send response with substituted id
            let mut our_response = Response::new(&authority_pen, &challenge).await;
            our_response.id = fake_id;
            send_data(stream, our_response).await.expect("should send");
            futures::future::pending::<()>().await;
        }

        let (stream_a, stream_b) = MockSplittable::new(4096);
        let (_, pen_a) = key().await;
        let (_, pen_b) = key().await;
        tokio::select! {
            result = execute_v0_handshake_incoming(stream_a, pen_a) => assert_signature_error(result),
            _ = execute_malicious_v0_handshake_outgoing_fake_signature(stream_b, pen_b) => panic!("should wait"),
        }
    }

    #[tokio::test]
    async fn broken_incoming_connection_step_one() {
        // break the connection even before the handshake starts by dropping the stream
        let (stream_a, _) = MockSplittable::new(4096);
        let (_, pen_a) = key().await;
        assert_send_error(execute_v0_handshake_incoming(stream_a, pen_a).await);
    }

    #[tokio::test]
    async fn broken_incoming_connection_step_two() {
        let (stream_a, stream_b) = MockSplittable::new(4096);
        let (_, pen_a) = key().await;
        let (result, _) = join!(
            execute_v0_handshake_incoming(stream_a, pen_a),
            // mock outgoing handshake: receive the first message and terminate
            async {
                receive_data::<_, Challenge>(stream_b)
                    .await
                    .expect("should receive");
            },
        );
        assert_receive_error(result);
    }

    #[tokio::test]
    async fn broken_outgoing_connection_step_one() {
        // break the connection even before the handshake starts by dropping the stream
        let (stream_a, _) = MockSplittable::new(4096);
        let (_, pen_a) = key().await;
        let (id_b, _) = key().await;
        assert_receive_error(execute_v0_handshake_outgoing(stream_a, pen_a, id_b).await);
    }

    #[tokio::test]
    async fn broken_outgoing_connection_step_two() {
        let (stream_a, stream_b) = MockSplittable::new(4096);
        let (id_a, pen_a) = key().await;
        let (_, pen_b) = key().await;
        // mock incoming handshake: send the first message and terminate
        send_data(stream_a, Challenge::new(pen_a.authority_id()))
            .await
            .expect("should send");
        assert_send_error(execute_v0_handshake_outgoing(stream_b, pen_b, id_a).await);
    }
}
