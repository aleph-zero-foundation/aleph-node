use std::fmt::{Debug, Display, Error as FmtError, Formatter};

use aleph_primitives::AuthorityId;
use futures::channel::mpsc;
use log::{debug, info};
use tokio::time::{sleep, Duration};

use crate::{
    crypto::AuthorityPen,
    validator_network::{
        protocol_negotiation::{protocol, ProtocolNegotiationError},
        protocols::ProtocolError,
        ConnectionInfo, Data, Dialer, PeerAddressInfo,
    },
};

enum OutgoingError<A: Data, ND: Dialer<A>> {
    Dial(ND::Error),
    ProtocolNegotiation(PeerAddressInfo, ProtocolNegotiationError),
    Protocol(PeerAddressInfo, ProtocolError),
}

impl<A: Data, ND: Dialer<A>> Display for OutgoingError<A, ND> {
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
        }
    }
}

async fn manage_outgoing<D: Data, A: Data, ND: Dialer<A>>(
    authority_pen: AuthorityPen,
    peer_id: AuthorityId,
    mut dialer: ND,
    addresses: Vec<A>,
    result_for_parent: mpsc::UnboundedSender<(AuthorityId, Option<mpsc::UnboundedSender<D>>)>,
) -> Result<(), OutgoingError<A, ND>> {
    debug!(target: "validator-network", "Trying to connect to {}.", peer_id);
    let stream = dialer
        .connect(addresses)
        .await
        .map_err(OutgoingError::Dial)?;
    let peer_address_info = stream.peer_address_info();
    debug!(target: "validator-network", "Performing outgoing protocol negotiation.");
    let (stream, protocol) = protocol(stream)
        .await
        .map_err(|e| OutgoingError::ProtocolNegotiation(peer_address_info.clone(), e))?;
    debug!(target: "validator-network", "Negotiated protocol, running.");
    protocol
        .manage_outgoing(stream, authority_pen, peer_id, result_for_parent)
        .await
        .map_err(|e| OutgoingError::Protocol(peer_address_info.clone(), e))
}

const RETRY_DELAY: Duration = Duration::from_secs(10);

/// Establish an outgoing connection to the provided peer using the dialer and then manage it.
/// While this works it will send any data from the user to the peer. Any failures will be reported
/// to the parent, so that connections can be reestablished if necessary.
pub async fn outgoing<D: Data, A: Data + Debug, ND: Dialer<A>>(
    authority_pen: AuthorityPen,
    peer_id: AuthorityId,
    dialer: ND,
    addresses: Vec<A>,
    result_for_parent: mpsc::UnboundedSender<(AuthorityId, Option<mpsc::UnboundedSender<D>>)>,
) {
    if let Err(e) = manage_outgoing(
        authority_pen,
        peer_id.clone(),
        dialer,
        addresses.clone(),
        result_for_parent.clone(),
    )
    .await
    {
        info!(target: "validator-network", "Outgoing connection to {} {:?} failed: {}, will retry after {}s.", peer_id, addresses, e, RETRY_DELAY.as_secs());
        sleep(RETRY_DELAY).await;
        if result_for_parent.unbounded_send((peer_id, None)).is_err() {
            debug!(target: "validator-network", "Could not send the closing message, we've probably been terminated by the parent service.");
        }
    }
}
