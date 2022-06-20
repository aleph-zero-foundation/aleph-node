use crate::{
    crypto::{Signature, SignatureV1},
    justification::AlephJustification,
};
use aleph_bft::{PartialMultisignature, SignatureSet};
use codec::{Decode, DecodeAll, Encode, Error as CodecError, Input as CodecInput};
use std::{
    fmt::{Display, Error as FmtError, Formatter},
    mem::size_of,
};

type Version = u16;
type ByteCount = u16;

/// Old format of justifications, needed for backwards compatibility.
/// Used an old format of signature which unnecessarily contained the signer ID.
#[derive(Clone, Encode, Decode, Debug, PartialEq, Eq)]
struct AlephJustificationV1 {
    pub signature: SignatureSet<SignatureV1>,
}

impl From<AlephJustificationV1> for AlephJustification {
    fn from(justification: AlephJustificationV1) -> AlephJustification {
        let size = justification.signature.size();
        let just_drop_id: SignatureSet<Signature> = justification
            .signature
            .into_iter()
            .fold(SignatureSet::with_size(size), |sig_set, (id, sgn)| {
                sig_set.add_signature(&sgn.into(), id)
            });
        AlephJustification {
            signature: just_drop_id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum VersionedAlephJustification {
    // Most likely from the future.
    Other(Version, Vec<u8>),
    V1(AlephJustificationV1),
    V2(AlephJustification),
}

fn encode_with_version(version: Version, mut payload: Vec<u8>) -> Vec<u8> {
    let mut result = version.encode();
    // This will produce rubbish if we ever try encodings that have more than u16::MAX bytes. We
    // expect this won't happen, since we will switch to proper multisignatures before proofs get
    // that big.
    let num_bytes = payload.len() as ByteCount;
    result.append(&mut num_bytes.encode());
    result.append(&mut payload);
    result
}

impl Encode for VersionedAlephJustification {
    fn size_hint(&self) -> usize {
        use VersionedAlephJustification::*;
        let version_size = size_of::<Version>();
        let byte_count_size = size_of::<ByteCount>();
        version_size
            + byte_count_size
            + match self {
                Other(_, payload) => payload.len(),
                V1(justification) => justification.size_hint(),
                V2(justification) => justification.size_hint(),
            }
    }

    fn encode(&self) -> Vec<u8> {
        use VersionedAlephJustification::*;
        match self {
            Other(version, payload) => encode_with_version(*version, payload.clone()),
            V1(justification) => encode_with_version(1, justification.encode()),
            V2(justification) => encode_with_version(2, justification.encode()),
        }
    }
}

impl Decode for VersionedAlephJustification {
    fn decode<I: CodecInput>(input: &mut I) -> Result<Self, CodecError> {
        use VersionedAlephJustification::*;
        let version = Version::decode(input)?;
        let num_bytes = ByteCount::decode(input)?;
        match version {
            1 => Ok(V1(AlephJustificationV1::decode(input)?)),
            2 => Ok(V2(AlephJustification::decode(input)?)),
            _ => {
                let mut payload = vec![0; num_bytes.into()];
                input.read(payload.as_mut_slice())?;
                Ok(Other(version, payload))
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    BadFormat,
    UnknownVersion(Version),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use Error::*;
        match self {
            BadFormat => write!(f, "malformed encoding"),
            UnknownVersion(version) => {
                write!(f, "justification encoded with unknown version {}", version)
            }
        }
    }
}

/// Decodes a justification, even if it was produced by ancient code which does not conform to our
/// backwards compatibility style.
pub fn backwards_compatible_decode(
    justification_raw: Vec<u8>,
) -> Result<AlephJustification, Error> {
    use Error::*;
    let justification_cloned = justification_raw.clone();
    match VersionedAlephJustification::decode_all(&mut justification_cloned.as_slice()) {
        Ok(justification) => {
            use VersionedAlephJustification::*;
            match justification {
                V1(justification) => Ok(justification.into()),
                V2(justification) => Ok(justification),
                Other(version, _) => Err(UnknownVersion(version)),
            }
        }
        Err(_) => {
            // We still have to be able to decode the pre-compatibility justifications, since they
            // may be lingering in the DB. Perhaps one day in the future we will be able to remove
            // this code, but I wouldn't count on it.
            let justification_cloned = justification_raw.clone();
            match AlephJustification::decode_all(&mut justification_cloned.as_slice()) {
                Ok(justification) => Ok(justification),
                Err(_) => match AlephJustificationV1::decode_all(&mut justification_raw.as_slice())
                {
                    Ok(justification) => Ok(justification.into()),
                    Err(_) => Err(BadFormat),
                },
            }
        }
    }
}

/// Encodes the justification in a way that is forwards compatible with future versions.
pub fn versioned_encode(justification: AlephJustification) -> Vec<u8> {
    VersionedAlephJustification::V2(justification).encode()
}

#[cfg(test)]
mod test {
    use super::{backwards_compatible_decode, AlephJustificationV1, VersionedAlephJustification};
    use crate::{
        crypto::{Signature, SignatureV1},
        justification::AlephJustification,
    };
    use aleph_bft::{NodeCount, PartialMultisignature, SignatureSet};
    use aleph_primitives::{AuthorityPair, AuthoritySignature};
    use codec::{Decode, Encode};
    use sp_core::Pair;

    #[test]
    fn correctly_decodes_v1() {
        let mut signature_set: SignatureSet<SignatureV1> = SignatureSet::with_size(7.into());
        for i in 0..7 {
            let id = i.into();
            let signature_v1 = SignatureV1 {
                _id: id,
                sgn: AuthorityPair::generate()
                    .0
                    .sign(vec![0u8, 0u8, 0u8, 0u8].as_slice()),
            };
            signature_set = signature_set.add_signature(&signature_v1, id);
        }

        let just_v1 = AlephJustificationV1 {
            signature: signature_set,
        };
        let encoded_just: Vec<u8> = just_v1.encode();
        let decoded = backwards_compatible_decode(encoded_just);
        let just_v1: AlephJustification = just_v1.into();
        assert_eq!(decoded, Ok(just_v1));
    }

    #[test]
    fn correctly_decodes_v2() {
        let mut signature_set: SignatureSet<Signature> = SignatureSet::with_size(7.into());
        for i in 0..7 {
            let authority_signature: AuthoritySignature = AuthorityPair::generate()
                .0
                .sign(vec![0u8, 0u8, 0u8, 0u8].as_slice());
            signature_set = signature_set.add_signature(&authority_signature.into(), i.into());
        }

        let just_v2 = AlephJustification {
            signature: signature_set,
        };
        let encoded_just: Vec<u8> = just_v2.encode();
        let decoded = backwards_compatible_decode(encoded_just);
        assert_eq!(decoded, Ok(just_v2));
    }

    #[test]
    fn correctly_decodes_other() {
        let other = VersionedAlephJustification::Other(43, vec![21, 37]);
        let encoded = other.encode();
        let decoded = VersionedAlephJustification::decode(&mut encoded.as_slice());
        assert_eq!(decoded, Ok(other));
    }

    #[test]
    fn correctly_decodes_legacy_v1_size4() {
        // This is a justification for 4 nodes generated by the version at commit `a426d7a`
        let raw: Vec<u8> = vec![
            16, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 70, 165, 218, 105, 238, 187, 137, 176, 148, 97, 251,
            204, 157, 166, 21, 31, 222, 144, 57, 47, 229, 130, 113, 221, 27, 138, 96, 189, 104, 39,
            235, 217, 107, 217, 184, 99, 252, 227, 142, 169, 60, 92, 64, 26, 128, 73, 40, 49, 208,
            54, 173, 47, 24, 229, 87, 93, 136, 157, 141, 173, 229, 156, 0, 1, 1, 0, 0, 0, 0, 0, 0,
            0, 148, 100, 171, 132, 5, 223, 228, 210, 92, 49, 165, 58, 241, 100, 3, 208, 81, 194,
            122, 36, 4, 31, 108, 104, 227, 164, 204, 165, 181, 237, 168, 93, 81, 37, 243, 183, 37,
            141, 233, 10, 232, 189, 189, 95, 129, 126, 113, 239, 121, 8, 18, 43, 200, 200, 128,
            211, 80, 34, 7, 104, 198, 215, 213, 8, 1, 2, 0, 0, 0, 0, 0, 0, 0, 126, 125, 118, 133,
            4, 152, 203, 42, 36, 177, 160, 243, 211, 223, 249, 171, 206, 112, 228, 170, 54, 6, 223,
            223, 83, 106, 27, 168, 40, 82, 48, 28, 150, 76, 98, 250, 13, 97, 163, 152, 77, 30, 153,
            206, 49, 210, 53, 218, 1, 52, 195, 97, 58, 229, 250, 198, 35, 155, 118, 249, 180, 123,
            12, 8, 0,
        ];
        match backwards_compatible_decode(raw) {
            Ok(justification) => assert_eq!(justification.signature.size(), NodeCount(4)),
            Err(e) => panic!("decoding V1 failed: {}", e),
        }
    }

    #[test]
    fn correctly_decodes_legacy_v1_size6() {
        // This is a justification for 6 nodes generated by the version at commit `a426d7a`
        let raw: Vec<u8> = vec![
            24, 1, 0, 0, 0, 0, 0, 0, 0, 0, 82, 120, 213, 50, 242, 152, 25, 224, 232, 243, 218, 52,
            111, 133, 171, 153, 160, 41, 16, 239, 33, 1, 252, 229, 53, 252, 155, 199, 63, 150, 6,
            227, 44, 130, 28, 24, 26, 202, 30, 197, 67, 119, 144, 44, 69, 39, 117, 53, 239, 104,
            165, 208, 143, 204, 4, 165, 6, 165, 27, 134, 7, 44, 172, 7, 1, 1, 0, 0, 0, 0, 0, 0, 0,
            173, 204, 199, 231, 18, 118, 216, 71, 19, 249, 239, 86, 196, 86, 173, 38, 113, 87, 118,
            112, 26, 70, 125, 228, 180, 101, 172, 159, 79, 8, 106, 247, 42, 255, 178, 0, 141, 194,
            242, 81, 93, 1, 230, 89, 247, 247, 233, 237, 136, 9, 254, 103, 74, 37, 43, 124, 226,
            59, 146, 242, 143, 208, 49, 13, 1, 2, 0, 0, 0, 0, 0, 0, 0, 162, 194, 14, 148, 20, 248,
            49, 230, 200, 102, 179, 223, 186, 103, 28, 58, 59, 67, 195, 77, 22, 20, 213, 92, 85,
            61, 133, 57, 55, 123, 221, 193, 121, 80, 18, 68, 92, 5, 2, 56, 55, 43, 1, 214, 145,
            131, 146, 103, 245, 135, 25, 251, 212, 85, 230, 223, 143, 44, 190, 102, 121, 121, 67,
            12, 1, 3, 0, 0, 0, 0, 0, 0, 0, 176, 17, 161, 159, 68, 184, 2, 127, 122, 162, 2, 213,
            232, 113, 111, 86, 35, 196, 150, 186, 221, 188, 14, 245, 41, 21, 206, 174, 134, 142,
            191, 212, 20, 102, 99, 111, 110, 48, 75, 214, 163, 173, 107, 251, 82, 24, 43, 131, 210,
            160, 59, 88, 111, 150, 236, 25, 128, 36, 179, 255, 56, 189, 99, 13, 1, 4, 0, 0, 0, 0,
            0, 0, 0, 140, 68, 206, 82, 199, 166, 235, 142, 114, 218, 219, 235, 206, 76, 253, 180,
            143, 213, 7, 39, 49, 154, 83, 142, 250, 26, 74, 37, 95, 106, 51, 179, 185, 75, 63, 244,
            63, 1, 179, 125, 53, 110, 220, 13, 126, 46, 124, 173, 98, 164, 194, 175, 52, 108, 43,
            68, 94, 254, 77, 39, 172, 255, 145, 10, 0,
        ];
        match backwards_compatible_decode(raw) {
            Ok(justification) => assert_eq!(justification.signature.size(), NodeCount(6)),
            Err(e) => panic!("decoding V1 failed: {}", e),
        }
    }
}
