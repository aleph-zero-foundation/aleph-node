use std::{
    fmt::{Display, Error as FmtError, Formatter},
    mem::size_of,
};

use codec::{Decode, Encode, Error as CodecError, Input as CodecInput};

use crate::network::{
    manager::{DiscoveryMessage, NetworkData},
    Data, Multiaddress,
};

type Version = u16;
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

fn encode_with_version(version: Version, mut payload: Vec<u8>) -> Vec<u8> {
    let mut result = version.encode();
    // This will produce rubbish if we ever try encodings that have more than u32::MAX bytes.
    let num_bytes = payload.len() as ByteCount;
    result.append(&mut num_bytes.encode());
    result.append(&mut payload);
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
            Other(version, payload) => encode_with_version(*version, payload.clone()),
            V1(data) => encode_with_version(1, data.encode()),
        }
    }
}

impl<M: Multiaddress> Decode for VersionedAuthentication<M> {
    fn decode<I: CodecInput>(input: &mut I) -> Result<Self, CodecError> {
        use VersionedAuthentication::*;
        let version = Version::decode(input)?;
        let num_bytes = ByteCount::decode(input)?;
        match version {
            1 => Ok(V1(DiscoveryMessage::decode(input)?)),
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
                write!(f, "Authentication has unknown version {}", version)
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
        SessionId,
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
        let other = VersionedAuthentication::<MockMultiaddress>::Other(42, vec![21, 37]);
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
                42,
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
