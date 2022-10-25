use std::{
    fmt::{Display, Error as FmtError, Formatter},
    mem::size_of,
};

use codec::{Decode, Encode, Error as CodecError, Input as CodecInput};
use log::warn;

use crate::{
    network::{
        manager::{DiscoveryMessage, NetworkData},
        Data, Multiaddress,
    },
    Version,
};

type ByteCount = u16;

// We allow sending authentications of size up to 16KiB, that should be enough.
const MAX_AUTHENTICATION_SIZE: u16 = 16 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VersionedAuthentication<M: Multiaddress> {
    // Most likely from the future.
    Other(Version, Vec<u8>),
    V1(DiscoveryMessage<M>),
}

impl<D: Data, M: Multiaddress> TryInto<NetworkData<D, M>> for VersionedAuthentication<M> {
    type Error = Error;

    fn try_into(self) -> Result<NetworkData<D, M>, Self::Error> {
        use VersionedAuthentication::*;
        match self {
            V1(message) => Ok(NetworkData::Meta(message)),
            Other(v, _) => Err(Error::UnknownVersion(v)),
        }
    }
}

impl<M: Multiaddress> From<DiscoveryMessage<M>> for VersionedAuthentication<M> {
    fn from(message: DiscoveryMessage<M>) -> VersionedAuthentication<M> {
        VersionedAuthentication::V1(message)
    }
}

fn encode_with_version(version: Version, payload: &[u8]) -> Vec<u8> {
    // If size is bigger then u16 we set it to MAX_AUTHENTICATION_SIZE.
    // This should never happen but in case it does we will not panic.
    // Also for other users if they have this version of protocol, authentication
    // will be decoded. If they do not know the protocol, authentication will result
    // in decoding error.
    // We do not have a guarantee that size_hint is implemented for DiscoveryMessage, so we need
    // to compute actual size to place it in the encoded data.
    let size = payload
        .len()
        .try_into()
        .unwrap_or(MAX_AUTHENTICATION_SIZE + 1);
    if size > MAX_AUTHENTICATION_SIZE {
        warn!(
            "Versioned Authentication v{:?} too big during Encode. Size is {:?}. Should be {:?} at max.",
            version,
            payload.len(),
            MAX_AUTHENTICATION_SIZE
        );
    }

    let mut result = Vec::with_capacity(version.size_hint() + size.size_hint() + payload.len());

    version.encode_to(&mut result);
    size.encode_to(&mut result);
    result.extend_from_slice(payload);

    result
}

impl<M: Multiaddress> Encode for VersionedAuthentication<M> {
    fn size_hint(&self) -> usize {
        use VersionedAuthentication::*;
        let version_size = size_of::<Version>();
        let byte_count_size = size_of::<ByteCount>();
        version_size
            + byte_count_size
            + match self {
                Other(_, payload) => payload.len(),
                V1(data) => data.size_hint(),
            }
    }

    fn encode(&self) -> Vec<u8> {
        use VersionedAuthentication::*;
        match self {
            Other(version, payload) => encode_with_version(*version, payload),
            V1(data) => encode_with_version(Version(1), &data.encode()),
        }
    }
}

impl<M: Multiaddress> Decode for VersionedAuthentication<M> {
    fn decode<I: CodecInput>(input: &mut I) -> Result<Self, CodecError> {
        use VersionedAuthentication::*;
        let version = Version::decode(input)?;
        let num_bytes = ByteCount::decode(input)?;
        match version {
            Version(1) => Ok(V1(DiscoveryMessage::decode(input)?)),
            _ => {
                if num_bytes > MAX_AUTHENTICATION_SIZE {
                    Err("Authentication has unknown version and is encoded as more than 16KiB.")?;
                };
                let mut payload = vec![0; num_bytes.into()];
                input.read(payload.as_mut_slice())?;
                Ok(Other(version, payload))
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    UnknownVersion(Version),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use Error::*;
        match self {
            UnknownVersion(version) => {
                write!(f, "Authentication has unknown version {}", version.0)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use codec::{Decode, Encode};

    use super::{DiscoveryMessage, VersionedAuthentication};
    use crate::{
        network::{
            manager::{compatibility::MAX_AUTHENTICATION_SIZE, SessionHandler},
            mock::{crypto_basics, MockMultiaddress, MockNetworkIdentity},
            NetworkIdentity,
        },
        SessionId, Version,
    };

    #[tokio::test]
    async fn correctly_decodes_v1() {
        let crypto_basics = crypto_basics(1).await;
        let handler = SessionHandler::new(
            Some(crypto_basics.0[0].clone()),
            crypto_basics.1.clone(),
            SessionId(43),
            MockNetworkIdentity::new().identity().0,
        )
        .await
        .unwrap();
        let authentication_v1 = VersionedAuthentication::V1(DiscoveryMessage::Authentication(
            handler.authentication().unwrap(),
        ));
        let encoded = authentication_v1.encode();
        let decoded = VersionedAuthentication::decode(&mut encoded.as_slice());
        assert_eq!(decoded, Ok(authentication_v1))
    }

    #[tokio::test]
    async fn correctly_decodes_other() {
        let other = VersionedAuthentication::<MockMultiaddress>::Other(Version(42), vec![21, 37]);
        let encoded = other.encode();
        let decoded = VersionedAuthentication::decode(&mut encoded.as_slice());
        assert_eq!(decoded, Ok(other));

        let mut other_big = 42u16.encode();
        other_big.append(&mut (MAX_AUTHENTICATION_SIZE).encode());
        other_big.append(&mut vec![0u8; (MAX_AUTHENTICATION_SIZE).into()]);
        let decoded =
            VersionedAuthentication::<MockMultiaddress>::decode(&mut other_big.as_slice());
        assert_eq!(
            decoded,
            Ok(VersionedAuthentication::<MockMultiaddress>::Other(
                Version(42),
                other_big[4..].to_vec()
            ))
        );
    }

    #[tokio::test]
    async fn returns_error_other_too_big() {
        let mut other = 42u16.encode();
        let size = MAX_AUTHENTICATION_SIZE + 1;
        other.append(&mut size.encode());
        other.append(&mut vec![0u8; size.into()]);
        let decoded = VersionedAuthentication::<MockMultiaddress>::decode(&mut other.as_slice());
        assert!(decoded.is_err());

        let other =
            VersionedAuthentication::<MockMultiaddress>::Other(Version(42), vec![0u8; size.into()]);
        let encoded = other.encode();
        let decoded = VersionedAuthentication::<MockMultiaddress>::decode(&mut encoded.as_slice());
        assert!(decoded.is_err());
    }

    #[tokio::test]
    async fn returns_error_other_wrong_size() {
        let mut other = 42u16.encode();
        other.append(&mut MAX_AUTHENTICATION_SIZE.encode());
        other.append(&mut vec![21, 37]);
        let decoded = VersionedAuthentication::<MockMultiaddress>::decode(&mut other.as_slice());
        assert!(decoded.is_err());
    }
}
