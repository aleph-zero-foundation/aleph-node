use std::{
    cmp::{max, min},
    fmt::{Display, Error as FmtError, Formatter},
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::{timeout, Duration},
};

use crate::protocols::{Protocol, Version};

const PROTOCOL_NEGOTIATION_TIMEOUT: Duration = Duration::from_secs(5);

/// A range of supported protocols, will fail to decode if the range is empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtocolsRange(Version, Version);

impl Display for ProtocolsRange {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(f, "[{},{}]", self.0, self.1)
    }
}

const fn supported_protocol_range() -> ProtocolsRange {
    ProtocolsRange(Protocol::MIN_VERSION, Protocol::MAX_VERSION)
}

/// What went wrong when negotiating a protocol.
#[derive(Debug, PartialEq, Eq)]
pub enum ProtocolNegotiationError {
    ConnectionClosed,
    InvalidRange(ProtocolsRange),
    ProtocolMismatch(ProtocolsRange, ProtocolsRange),
    BadChoice(Version),
    TimedOut,
}

impl Display for ProtocolNegotiationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use ProtocolNegotiationError::*;
        match self {
            ConnectionClosed => write!(f, "connection closed"),
            InvalidRange(range) => write!(f, "invalid range: {}", range),
            ProtocolMismatch(our_range, their_range) => write!(
                f,
                "failed negotiation with range {}, their {}",
                our_range, their_range
            ),
            BadChoice(version) => write!(
                f,
                "negotiated protocol version {}, which we don't know, this is a severe bug",
                version
            ),
            TimedOut => write!(f, "timed out"),
        }
    }
}

impl ProtocolsRange {
    fn valid(&self) -> bool {
        self.0 <= self.1
    }

    fn encode(&self) -> [u8; 8] {
        let mut result = self.0.to_le_bytes().to_vec();
        result.append(&mut self.1.to_le_bytes().to_vec());
        result.try_into().expect("this is literally 8 bytes")
    }

    fn decode(encoded: &[u8; 8]) -> Result<Self, ProtocolNegotiationError> {
        let result = ProtocolsRange(
            Version::from_le_bytes(encoded[0..4].try_into().expect("this is literally 4 bytes")),
            Version::from_le_bytes(encoded[4..8].try_into().expect("this is literally 4 bytes")),
        );
        match result.valid() {
            true => Ok(result),
            false => Err(ProtocolNegotiationError::InvalidRange(result)),
        }
    }
}

fn intersection(
    range1: ProtocolsRange,
    range2: ProtocolsRange,
) -> Result<ProtocolsRange, ProtocolNegotiationError> {
    let intersection = ProtocolsRange(max(range1.0, range2.0), min(range1.1, range2.1));
    match intersection.valid() {
        true => Ok(intersection),
        false => Err(ProtocolNegotiationError::ProtocolMismatch(range1, range2)),
    }
}

fn maximum_of_intersection(
    range1: ProtocolsRange,
    range2: ProtocolsRange,
) -> Result<Protocol, ProtocolNegotiationError> {
    intersection(range1, range2).map(|intersection| {
        intersection
            .1
            .try_into()
            .map_err(ProtocolNegotiationError::BadChoice)
    })?
}

async fn negotiate_protocol_version<S: AsyncReadExt + AsyncWriteExt + Unpin>(
    mut stream: S,
    our_protocol_range: ProtocolsRange,
) -> Result<(S, Protocol), ProtocolNegotiationError> {
    stream
        .write_all(&our_protocol_range.encode())
        .await
        .map_err(|_| ProtocolNegotiationError::ConnectionClosed)?;
    let mut buf = [0; 8];
    stream
        .read_exact(&mut buf)
        .await
        .map_err(|_| ProtocolNegotiationError::ConnectionClosed)?;
    let their_protocol_range = ProtocolsRange::decode(&buf)?;
    Ok((
        stream,
        maximum_of_intersection(our_protocol_range, their_protocol_range)?,
    ))
}

/// Negotiate a protocol version to use.
pub async fn protocol<S: AsyncReadExt + AsyncWriteExt + Unpin>(
    stream: S,
) -> Result<(S, Protocol), ProtocolNegotiationError> {
    timeout(
        PROTOCOL_NEGOTIATION_TIMEOUT,
        negotiate_protocol_version(stream, supported_protocol_range()),
    )
    .await
    .map_err(|_| ProtocolNegotiationError::TimedOut)?
}

#[cfg(test)]
mod tests {
    use futures::{pin_mut, FutureExt};
    use tokio::io::duplex;

    use super::{negotiate_protocol_version, supported_protocol_range, ProtocolNegotiationError};
    use crate::protocols::Protocol;

    fn correct_negotiation<S>(result: Result<(S, Protocol), ProtocolNegotiationError>) {
        match result {
            Ok((_stream, protocol)) => assert_eq!(Protocol::V1, protocol),
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    fn incorrect_negotiation<S>(
        result: Result<(S, Protocol), ProtocolNegotiationError>,
        expected_error: ProtocolNegotiationError,
    ) {
        match result {
            Ok((_stream, protocol)) => {
                panic!("Unexpectedly managed to negotiate protocol {:?}", protocol)
            }
            Err(e) => assert_eq!(expected_error, e),
        }
    }

    #[tokio::test]
    async fn negotiates_when_both_agree_exactly() {
        let (stream1, stream2) = duplex(4096);
        let negotiation1 = negotiate_protocol_version(stream1, supported_protocol_range()).fuse();
        pin_mut!(negotiation1);
        let negotiation2 = negotiate_protocol_version(stream2, supported_protocol_range()).fuse();
        pin_mut!(negotiation2);
        for _ in 0..2 {
            tokio::select! {
                result = &mut negotiation1 => correct_negotiation(result),
                result = &mut negotiation2 => correct_negotiation(result),
            }
        }
    }

    #[tokio::test]
    async fn negotiates_when_one_broader() {
        let (stream1, stream2) = duplex(4096);
        let mut broader_protocol_range = supported_protocol_range();
        broader_protocol_range.1 += 1;
        let negotiation1 = negotiate_protocol_version(stream1, supported_protocol_range()).fuse();
        pin_mut!(negotiation1);
        let negotiation2 = negotiate_protocol_version(stream2, broader_protocol_range).fuse();
        pin_mut!(negotiation2);
        for _ in 0..2 {
            tokio::select! {
                result = &mut negotiation1 => correct_negotiation(result),
                result = &mut negotiation2 => correct_negotiation(result),
            }
        }
    }

    #[tokio::test]
    async fn fails_when_no_intersection() {
        let (stream1, stream2) = duplex(4096);
        let mut too_high_protocol_range = supported_protocol_range();
        too_high_protocol_range.0 = too_high_protocol_range.1 + 1;
        too_high_protocol_range.1 = too_high_protocol_range.0 + 1;
        let negotiation1 = negotiate_protocol_version(stream1, supported_protocol_range()).fuse();
        pin_mut!(negotiation1);
        let negotiation2 =
            negotiate_protocol_version(stream2, too_high_protocol_range.clone()).fuse();
        pin_mut!(negotiation2);
        for _ in 0..2 {
            tokio::select! {
                result = &mut negotiation1 => incorrect_negotiation(result, ProtocolNegotiationError::ProtocolMismatch(supported_protocol_range(), too_high_protocol_range.clone())),
                result = &mut negotiation2 => incorrect_negotiation(result, ProtocolNegotiationError::ProtocolMismatch(too_high_protocol_range.clone(), supported_protocol_range())),
            }
        }
    }

    #[tokio::test]
    async fn fails_when_bad_negotiation() {
        let (stream1, stream2) = duplex(4096);
        let mut too_high_protocol_range = supported_protocol_range();
        too_high_protocol_range.0 = too_high_protocol_range.1 + 1;
        too_high_protocol_range.1 = too_high_protocol_range.0 + 1;
        let negotiation1 =
            negotiate_protocol_version(stream1, too_high_protocol_range.clone()).fuse();
        pin_mut!(negotiation1);
        let negotiation2 =
            negotiate_protocol_version(stream2, too_high_protocol_range.clone()).fuse();
        pin_mut!(negotiation2);
        for _ in 0..2 {
            tokio::select! {
                result = &mut negotiation1 => incorrect_negotiation(result, ProtocolNegotiationError::BadChoice(too_high_protocol_range.1)),
                result = &mut negotiation2 => incorrect_negotiation(result, ProtocolNegotiationError::BadChoice(too_high_protocol_range.1)),
            }
        }
    }

    #[tokio::test]
    async fn fails_when_invalid_range() {
        let (stream1, stream2) = duplex(4096);
        let mut invalid_range = supported_protocol_range();
        invalid_range.0 = invalid_range.1 + 1;
        let negotiation1 = negotiate_protocol_version(stream1, invalid_range.clone()).fuse();
        pin_mut!(negotiation1);
        let negotiation2 = negotiate_protocol_version(stream2, invalid_range.clone()).fuse();
        pin_mut!(negotiation2);
        for _ in 0..2 {
            tokio::select! {
                result = &mut negotiation1 => incorrect_negotiation(result, ProtocolNegotiationError::InvalidRange(invalid_range.clone())),
                result = &mut negotiation2 => incorrect_negotiation(result, ProtocolNegotiationError::InvalidRange(invalid_range.clone())),
            }
        }
    }

    #[tokio::test]
    async fn fails_when_connection_dropped() {
        let (stream, _) = duplex(4096);
        incorrect_negotiation(
            negotiate_protocol_version(stream, supported_protocol_range()).await,
            ProtocolNegotiationError::ConnectionClosed,
        );
    }
}
