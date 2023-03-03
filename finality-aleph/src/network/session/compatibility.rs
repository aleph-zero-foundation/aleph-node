use std::{
    fmt::{Display, Error as FmtError, Formatter},
    mem::size_of,
};

use codec::{Decode, Encode, Error as CodecError, Input as CodecInput};
use log::warn;

use crate::{
    network::{session::Authentication, AddressingInformation},
    SessionId, Version,
};

type ByteCount = u16;

// We allow sending authentications of size up to 16KiB, that should be enough.
const MAX_AUTHENTICATION_SIZE: u16 = 16 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VersionedAuthentication<A: AddressingInformation> {
    // Most likely from the future.
    Other(Version, Vec<u8>),
    V2(Authentication<A>),
}

impl<A: AddressingInformation> From<Authentication<A>> for Vec<VersionedAuthentication<A>> {
    fn from(authentication: Authentication<A>) -> Self {
        vec![VersionedAuthentication::V2(authentication)]
    }
}

pub type DiscoveryMessage<A> = Authentication<A>;

impl<A: AddressingInformation> DiscoveryMessage<A> {
    /// Session ID associated with this message.
    pub fn session_id(&self) -> SessionId {
        self.0.session()
    }
}

impl<A: AddressingInformation> TryInto<DiscoveryMessage<A>> for VersionedAuthentication<A> {
    type Error = Error;

    fn try_into(self) -> Result<DiscoveryMessage<A>, Self::Error> {
        use VersionedAuthentication::*;
        match self {
            V2(authentication) => Ok(authentication),
            Other(v, _) => Err(Error::UnknownVersion(v)),
        }
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

impl<A: AddressingInformation> Encode for VersionedAuthentication<A> {
    fn size_hint(&self) -> usize {
        use VersionedAuthentication::*;
        let version_size = size_of::<Version>();
        let byte_count_size = size_of::<ByteCount>();
        version_size
            + byte_count_size
            + match self {
                Other(_, payload) => payload.len(),
                V2(data) => data.size_hint(),
            }
    }

    fn encode(&self) -> Vec<u8> {
        use VersionedAuthentication::*;
        match self {
            Other(version, payload) => encode_with_version(*version, payload),
            V2(data) => encode_with_version(Version(2), &data.encode()),
        }
    }
}

impl<A: AddressingInformation> Decode for VersionedAuthentication<A> {
    fn decode<I: CodecInput>(input: &mut I) -> Result<Self, CodecError> {
        use VersionedAuthentication::*;
        let version = Version::decode(input)?;
        let num_bytes = ByteCount::decode(input)?;
        match version {
            Version(2) => Ok(V2(Authentication::decode(input)?)),
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

    use super::VersionedAuthentication;
    use crate::{
        crypto::AuthorityVerifier,
        network::{
            clique::mock::MockAddressingInformation,
            session::{compatibility::MAX_AUTHENTICATION_SIZE, SessionHandler},
            tcp::{testing::new_identity, SignedTcpAddressingInformation},
            NetworkIdentity,
        },
        nodes::testing::new_pen,
        NodeIndex, SessionId, Version,
    };

    /// Session Handler used for generating versioned authentication in `raw_authentication_v1`
    async fn handler() -> SessionHandler<SignedTcpAddressingInformation> {
        let mnemonic = "ring cool spatial rookie need wing opinion pond fork garbage more april";
        let external_addresses = vec![
            String::from("addr1"),
            String::from("addr2"),
            String::from("addr3"),
        ];

        let keystore = Arc::new(KeyStore::new());
        let pen = new_pen(mnemonic, keystore).await;
        let identity = new_identity(external_addresses, &pen).await;

        SessionHandler::new(
            Some((NodeIndex(21), pen)),
            AuthorityVerifier::new(vec![]),
            SessionId(37),
            identity.identity(),
        )
        .await
    }

    fn authentication_v2(
        handler: SessionHandler<SignedTcpAddressingInformation>,
    ) -> VersionedAuthentication<SignedTcpAddressingInformation> {
        VersionedAuthentication::V2(
            handler
                .authentication()
                .expect("should have authentication"),
        )
    }

    /// Versioned authentication for authority with:
    /// external_addresses: [String::from("addr1"), String::from("addr2"), String::from("addr3")]
    /// derived from mnemonic "ring cool spatial rookie need wing opinion pond fork garbage more april"
    /// for node index 21 and session id 37
    /// encoded at version of Aleph Node after 8.0
    fn raw_authentication_v2() -> Vec<u8> {
        //TODO: this will fail, check what it should be
        vec![
            2, 0, 191, 0, 50, 40, 192, 239, 72, 72, 119, 156, 76, 37, 212, 220, 76, 165, 39, 73,
            20, 89, 77, 66, 171, 174, 61, 31, 254, 137, 186, 1, 7, 141, 187, 219, 20, 97, 100, 100,
            114, 49, 8, 20, 97, 100, 100, 114, 50, 20, 97, 100, 100, 114, 51, 193, 134, 174, 215,
            223, 67, 113, 105, 253, 217, 120, 59, 47, 176, 146, 72, 205, 114, 242, 242, 115, 214,
            97, 112, 69, 56, 119, 168, 164, 170, 74, 7, 97, 149, 53, 122, 42, 209, 198, 146, 6,
            169, 37, 242, 131, 152, 209, 10, 52, 78, 218, 52, 69, 81, 235, 254, 58, 44, 134, 201,
            119, 132, 5, 8, 21, 0, 0, 0, 0, 0, 0, 0, 37, 0, 0, 0, 230, 134, 124, 175, 213, 131, 76,
            99, 89, 247, 169, 129, 87, 134, 249, 172, 99, 77, 203, 254, 12, 171, 178, 163, 47, 145,
            104, 166, 75, 174, 164, 119, 197, 78, 101, 221, 52, 51, 116, 221, 67, 45, 196, 65, 61,
            5, 246, 111, 56, 215, 145, 48, 170, 241, 60, 68, 231, 187, 72, 201, 18, 82, 249, 11,
        ]
    }

    #[tokio::test]
    async fn correcly_encodes_v2_to_bytes() {
        let handler = handler().await;
        let raw = raw_authentication_v2();
        let authentication_v2 = authentication_v2(handler);

        assert_eq!(authentication_v2.encode(), raw);
    }

    #[tokio::test]
    async fn correcly_decodes_v2_from_bytes() {
        let handler = handler().await;
        let raw = raw_authentication_v2();
        let authentication_v2 = authentication_v2(handler);

        let decoded = VersionedAuthentication::decode(&mut raw.as_slice());

        assert_eq!(decoded, Ok(authentication_v2));
    }

    #[tokio::test]
    async fn correctly_decodes_v2_roundtrip() {
        let handler = handler().await;
        let authentication_v2 = authentication_v2(handler);

        let encoded = authentication_v2.encode();
        let decoded = VersionedAuthentication::decode(&mut encoded.as_slice());

        assert_eq!(decoded, Ok(authentication_v2))
    }

    #[tokio::test]
    async fn correctly_decodes_other() {
        let other =
            VersionedAuthentication::<MockAddressingInformation>::Other(Version(42), vec![21, 37]);
        let encoded = other.encode();
        let decoded = VersionedAuthentication::decode(&mut encoded.as_slice());
        assert_eq!(decoded, Ok(other));

        let mut other_big = 42u16.encode();
        other_big.append(&mut (MAX_AUTHENTICATION_SIZE).encode());
        other_big.append(&mut vec![0u8; (MAX_AUTHENTICATION_SIZE).into()]);
        let decoded =
            VersionedAuthentication::<MockAddressingInformation>::decode(&mut other_big.as_slice());
        assert_eq!(
            decoded,
            Ok(VersionedAuthentication::<MockAddressingInformation>::Other(
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
        let decoded =
            VersionedAuthentication::<MockAddressingInformation>::decode(&mut other.as_slice());
        assert!(decoded.is_err());

        let other = VersionedAuthentication::<MockAddressingInformation>::Other(
            Version(42),
            vec![0u8; size.into()],
        );
        let encoded = other.encode();
        let decoded =
            VersionedAuthentication::<MockAddressingInformation>::decode(&mut encoded.as_slice());
        assert!(decoded.is_err());
    }

    #[tokio::test]
    async fn returns_error_other_wrong_size() {
        let mut other = 42u16.encode();
        other.append(&mut MAX_AUTHENTICATION_SIZE.encode());
        other.append(&mut vec![21, 37]);
        let decoded =
            VersionedAuthentication::<MockAddressingInformation>::decode(&mut other.as_slice());
        assert!(decoded.is_err());
    }
}
