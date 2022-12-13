use std::{
    fmt::{Display, Error as FmtError, Formatter},
    mem::size_of,
};

use codec::{Decode, Encode, Error as CodecError, Input as CodecInput};
use log::warn;

use crate::{
    network::{manager::DiscoveryMessage, Multiaddress},
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

impl<M: Multiaddress> TryInto<DiscoveryMessage<M>> for VersionedAuthentication<M> {
    type Error = Error;

    fn try_into(self) -> Result<DiscoveryMessage<M>, Self::Error> {
        use VersionedAuthentication::*;
        match self {
            V1(message) => Ok(message),
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
    use std::sync::Arc;

    use codec::{Decode, Encode};
    use sp_keystore::testing::KeyStore;

    use super::{DiscoveryMessage, VersionedAuthentication};
    use crate::{
        crypto::AuthorityVerifier,
        network::{
            manager::{compatibility::MAX_AUTHENTICATION_SIZE, SessionHandler},
            NetworkIdentity,
        },
        nodes::testing::new_pen,
        tcp_network::{testing::new_identity, TcpMultiaddress},
        testing::mocks::validator_network::MockMultiaddress,
        NodeIndex, SessionId, Version,
    };

    /// Session Handler used for generating versioned authentication in `raw_authentication_v1`
    async fn handler() -> SessionHandler<TcpMultiaddress> {
        let mnemonic = "ring cool spatial rookie need wing opinion pond fork garbage more april";
        let external_addresses = vec![
            String::from("addr1"),
            String::from("addr2"),
            String::from("addr3"),
        ];

        let keystore = Arc::new(KeyStore::new());
        let pen = new_pen(mnemonic, keystore).await;
        let identity = new_identity(
            external_addresses.into_iter().map(String::from).collect(),
            pen.authority_id(),
        );

        SessionHandler::new(
            Some((NodeIndex(21), pen)),
            AuthorityVerifier::new(vec![]),
            SessionId(37),
            identity.identity().0,
        )
        .await
        .unwrap()
    }

    /// Versioned authentication for authority with:
    /// external_addresses: [String::from("addr1"), String::from("addr2"), String::from("addr3")]
    /// derived from mnemonic "ring cool spatial rookie need wing opinion pond fork garbage more april"
    /// for node index 21 and session id 37
    /// encoded at version of Aleph Node from r-8.0
    fn raw_authentication_v1() -> Vec<u8> {
        vec![
            1, 0, 192, 0, 1, 12, 50, 40, 192, 239, 72, 72, 119, 156, 76, 37, 212, 220, 76, 165, 39,
            73, 20, 89, 77, 66, 171, 174, 61, 31, 254, 137, 186, 1, 7, 141, 187, 219, 20, 97, 100,
            100, 114, 49, 50, 40, 192, 239, 72, 72, 119, 156, 76, 37, 212, 220, 76, 165, 39, 73,
            20, 89, 77, 66, 171, 174, 61, 31, 254, 137, 186, 1, 7, 141, 187, 219, 20, 97, 100, 100,
            114, 50, 50, 40, 192, 239, 72, 72, 119, 156, 76, 37, 212, 220, 76, 165, 39, 73, 20, 89,
            77, 66, 171, 174, 61, 31, 254, 137, 186, 1, 7, 141, 187, 219, 20, 97, 100, 100, 114,
            51, 21, 0, 0, 0, 0, 0, 0, 0, 37, 0, 0, 0, 166, 39, 166, 74, 57, 190, 80, 240, 169, 85,
            240, 126, 250, 119, 54, 24, 244, 91, 199, 127, 32, 78, 52, 98, 159, 182, 227, 170, 251,
            49, 47, 89, 13, 171, 79, 190, 220, 22, 65, 254, 25, 115, 232, 103, 177, 252, 161, 222,
            74, 18, 216, 213, 105, 220, 223, 247, 221, 85, 31, 146, 177, 96, 254, 9,
        ]
    }

    #[tokio::test]
    async fn correcly_encodes_v1_to_bytes() {
        let handler = handler().await;
        let raw = raw_authentication_v1();

        let authentication_v1 = VersionedAuthentication::V1(DiscoveryMessage::Authentication(
            handler.authentication().unwrap(),
        ));

        assert_eq!(authentication_v1.encode(), raw);
    }

    #[tokio::test]
    async fn correcly_decodes_v1_from_bytes() {
        let handler = handler().await;
        let raw = raw_authentication_v1();

        let authentication_v1 = VersionedAuthentication::V1(DiscoveryMessage::Authentication(
            handler.authentication().unwrap(),
        ));

        let decoded = VersionedAuthentication::decode(&mut raw.as_slice());

        assert_eq!(decoded, Ok(authentication_v1));
    }

    #[tokio::test]
    async fn correctly_decodes_v1_roundtrip() {
        let handler = handler().await;

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
