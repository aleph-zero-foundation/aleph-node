use std::fmt::{Debug, Display, Error as FmtError, Formatter};

use futures::channel::mpsc;
use log::{debug, info};
use tokio::time::{sleep, timeout, Duration};

use crate::network::clique::{
    protocols::{protocol, ProtocolError, ProtocolNegotiationError, ResultForService},
    ConnectionInfo, Data, Dialer, PeerAddressInfo, PublicKey, SecretKey, LOG_TARGET,
};

enum OutgoingError<PK: PublicKey, A: Data, ND: Dialer<A>> {
    Dial(ND::Error),
    ProtocolNegotiation(PeerAddressInfo, ProtocolNegotiationError),
    Protocol(PeerAddressInfo, ProtocolError<PK>),
    TimedOut,
}

impl<PK: PublicKey, A: Data, ND: Dialer<A>> Display for OutgoingError<PK, A, ND> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use OutgoingError::*;
        match self {
            Dial(e) => write!(f, "dial error: {}", e),
            ProtocolNegotiation(addr, e) => write!(
                f,
                "communication with {} failed, protocol negotiation error: {}",
                addr, e
            ),
            Protocol(addr, e) => write!(
                f,
                "communication with {} failed, protocol error: {}",
                addr, e
            ),
            TimedOut => write!(f, "dial timeout",),
        }
    }
}

/// Arbitrarily chosen timeout, should be more than enough.
const DIAL_TIMEOUT: Duration = Duration::from_secs(60);

async fn manage_outgoing<SK: SecretKey, D: Data, A: Data, ND: Dialer<A>>(
    secret_key: SK,
    public_key: SK::PublicKey,
    mut dialer: ND,
    address: A,
    result_for_parent: mpsc::UnboundedSender<ResultForService<SK::PublicKey, D>>,
    data_for_user: mpsc::UnboundedSender<D>,
) -> Result<(), OutgoingError<SK::PublicKey, A, ND>> {
    debug!(target: LOG_TARGET, "Trying to connect to {}.", public_key);
    let stream = timeout(DIAL_TIMEOUT, dialer.connect(address))
        .await
        .map_err(|_| OutgoingError::TimedOut)?
        .map_err(OutgoingError::Dial)?;
    let peer_address_info = stream.peer_address_info();
    debug!(
        target: LOG_TARGET,
        "Performing outgoing protocol negotiation."
    );
    let (stream, protocol) = protocol(stream)
        .await
        .map_err(|e| OutgoingError::ProtocolNegotiation(peer_address_info.clone(), e))?;
    debug!(target: LOG_TARGET, "Negotiated protocol, running.");
    protocol
        .manage_outgoing(
            stream,
            secret_key,
            public_key,
            result_for_parent,
            data_for_user,
        )
        .await
        .map_err(|e| OutgoingError::Protocol(peer_address_info.clone(), e))
}

const RETRY_DELAY: Duration = Duration::from_secs(10);

/// Establish an outgoing connection to the provided peer using the dialer and then manage it.
/// While this works it will send any data from the user to the peer. Any failures will be reported
/// to the parent, so that connections can be reestablished if necessary.
pub async fn outgoing<SK: SecretKey, D: Data, A: Data + Debug, ND: Dialer<A>>(
    secret_key: SK,
    public_key: SK::PublicKey,
    dialer: ND,
    address: A,
    result_for_parent: mpsc::UnboundedSender<ResultForService<SK::PublicKey, D>>,
    data_for_user: mpsc::UnboundedSender<D>,
) {
    if let Err(e) = manage_outgoing(
        secret_key,
        public_key.clone(),
        dialer,
        address.clone(),
        result_for_parent.clone(),
        data_for_user,
    )
    .await
    {
        info!(
            target: LOG_TARGET,
            "Outgoing connection to {} {:?} failed: {}, will retry after {}s.",
            public_key,
            address,
            e,
            RETRY_DELAY.as_secs()
        );
        sleep(RETRY_DELAY).await;
        // we send the "new" connection type, because we always assume it's new until proven
        // otherwise, and here we did not even get the chance to attempt negotiating a protocol
        if result_for_parent
            .unbounded_send((public_key, None))
            .is_err()
        {
            debug!(target: LOG_TARGET, "Could not send the closing message, we've probably been terminated by the parent service.");
        }
    }
}
