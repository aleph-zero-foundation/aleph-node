use std::{
    fmt::{Display, Error as FmtError, Formatter},
    mem::size_of,
};

use log::warn;
use parity_scale_codec::{Decode, DecodeAll, Encode, Error as CodecError, Input as CodecInput};

use crate::{
    abft::SignatureSet,
    crypto::{Signature, SignatureV1},
    justification::{AlephJustification, LOG_TARGET},
    Version,
};

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
        AlephJustification::CommitteeMultisignature(just_drop_id)
    }
}

/// Old format of justifications, needed for backwards compatibility.
/// Used an old format of signature from before the compatibility changes.
#[derive(Clone, Encode, Decode, Debug, PartialEq, Eq)]
struct AlephJustificationV2 {
    pub signature: SignatureSet<Signature>,
}

impl From<AlephJustificationV2> for AlephJustification {
    fn from(justification: AlephJustificationV2) -> AlephJustification {
        AlephJustification::CommitteeMultisignature(justification.signature)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum VersionedAlephJustification {
    // Most likely from the future.
    Other(Version, Vec<u8>),
    V1(AlephJustificationV1),
    V2(AlephJustificationV2),
    V3(AlephJustification),
}

fn encode_with_version(version: Version, payload: &[u8]) -> Vec<u8> {
    // This will produce rubbish if we ever try encodings that have more than u16::MAX bytes. We
    // expect this won't happen, since we will switch to proper multisignatures before proofs get
    // that big.
    // We do not have a guarantee that size_hint is implemented for AlephJustification, so we need
    // to compute actual size to place it in the encoded data.
    let size = payload.len().try_into().unwrap_or_else(|_| {
        if payload.len() > ByteCount::MAX.into() {
            warn!(
                target: LOG_TARGET,
                "Versioned Justification v{:?} too big during Encode. Size is {:?}. Should be {:?} at max.",
                version,
                payload.len(),
                ByteCount::MAX
            );
        }
        ByteCount::MAX
    });

    let mut result = Vec::with_capacity(version.size_hint() + size.size_hint() + payload.len());

    version.encode_to(&mut result);
    size.encode_to(&mut result);
    result.extend_from_slice(payload);

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
                V3(justification) => justification.size_hint(),
            }
    }

    fn encode(&self) -> Vec<u8> {
        use VersionedAlephJustification::*;
        match self {
            Other(version, payload) => encode_with_version(*version, payload),
            V1(justification) => encode_with_version(Version(1), &justification.encode()),
            V2(justification) => encode_with_version(Version(2), &justification.encode()),
            V3(justification) => encode_with_version(Version(3), &justification.encode()),
        }
    }
}

impl Decode for VersionedAlephJustification {
    fn decode<I: CodecInput>(input: &mut I) -> Result<Self, CodecError> {
        use VersionedAlephJustification::*;
        let version = Version::decode(input)?;
        let num_bytes = ByteCount::decode(input)?;
        match version {
            Version(1) => Ok(V1(AlephJustificationV1::decode(input)?)),
            Version(2) => Ok(V2(AlephJustificationV2::decode(input)?)),
            Version(3) => Ok(V3(AlephJustification::decode(input)?)),
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
                write!(
                    f,
                    "justification encoded with unknown version {}",
                    version.0
                )
            }
        }
    }
}

fn decode_pre_compatibility_justification(
    justification_raw: Vec<u8>,
) -> Result<AlephJustification, Error> {
    use Error::*;

    // We still have to be able to decode the pre-compatibility justifications, since they
    // may be lingering in the DB. Perhaps one day in the future we will be able to remove
    // this code, but I wouldn't count on it.
    match AlephJustificationV2::decode_all(&mut justification_raw.as_slice()) {
        Ok(justification) => Ok(justification.into()),
        Err(_) => match AlephJustificationV1::decode_all(&mut justification_raw.as_slice()) {
            Ok(justification) => Ok(justification.into()),
            Err(_) => Err(BadFormat),
        },
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
                V2(justification) => Ok(justification.into()),
                V3(justification) => Ok(justification),
                Other(version, _) => {
                    // it is a coincidence that sometimes pre-compatibility legacy justification second word,
                    // which is in VersionedAlephJustification byte_count_size, can be small enough
                    // so that justification is false positively recognized  as from the future
                    // therefore we should try to decode formats
                    decode_pre_compatibility_justification(justification_raw)
                        .map_err(|_| UnknownVersion(version))
                }
            }
        }
        Err(_) => decode_pre_compatibility_justification(justification_raw),
    }
}

/// Encodes the justification in a way that is forwards compatible with future versions.
pub fn versioned_encode(justification: AlephJustification) -> Vec<u8> {
    VersionedAlephJustification::V3(justification).encode()
}

#[cfg(test)]
mod test {
    use parity_scale_codec::{Decode, Encode};
    use sp_core::Pair;

    use super::{
        backwards_compatible_decode, versioned_encode, AlephJustificationV1, AlephJustificationV2,
        VersionedAlephJustification,
    };
    use crate::{
        aleph_primitives::{AuthorityPair, AuthoritySignature},
        crypto::{Signature, SignatureV1},
        justification::AlephJustification,
        NodeCount, SignatureSet, Version,
    };

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

        let just_v2 = AlephJustificationV2 {
            signature: signature_set,
        };
        let encoded_just: Vec<u8> = just_v2.encode();
        let decoded = backwards_compatible_decode(encoded_just);
        let just_v2: AlephJustification = just_v2.into();
        assert_eq!(decoded, Ok(just_v2));
    }

    #[test]
    fn correctly_decodes_v3_committee() {
        let mut signature_set: SignatureSet<Signature> = SignatureSet::with_size(7.into());
        for i in 0..7 {
            let authority_signature: AuthoritySignature = AuthorityPair::generate()
                .0
                .sign(vec![0u8, 0u8, 0u8, 0u8].as_slice());
            signature_set = signature_set.add_signature(&authority_signature.into(), i.into());
        }

        let just_v3 = AlephJustification::CommitteeMultisignature(signature_set);
        // Here we use `versioned_encode` since we never sent plain v3 justifications.
        let encoded_just = versioned_encode(just_v3.clone());
        let decoded = backwards_compatible_decode(encoded_just);
        assert_eq!(decoded, Ok(just_v3));
    }

    #[test]
    fn correctly_decodes_other() {
        let other = VersionedAlephJustification::Other(Version(43), vec![21, 37]);
        let encoded = other.encode();
        let decoded = VersionedAlephJustification::decode(&mut encoded.as_slice());
        assert_eq!(decoded, Ok(other));
    }

    fn assert_backwards_compatible_decodes_pre_compatibility_justification(
        raw_justification_legacy_pre_compatibility: Vec<u8>,
        expected_node_count: usize,
    ) {
        match backwards_compatible_decode(raw_justification_legacy_pre_compatibility) {
            Ok(AlephJustification::CommitteeMultisignature(signature)) => {
                assert_eq!(signature.size(), NodeCount(expected_node_count))
            }
            Ok(AlephJustification::EmergencySignature(_)) => {
                panic!("decoded V1 as emergency signature")
            }
            Err(e) => panic!("decoding V1 failed: {e}"),
        }
    }

    #[test]
    fn correctly_decodes_legacy_pre_compatibility_size4() {
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
        assert_backwards_compatible_decodes_pre_compatibility_justification(raw, 4);
    }

    #[test]
    fn correctly_decodes_legacy_pre_compatibility_size6() {
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
        assert_backwards_compatible_decodes_pre_compatibility_justification(raw, 6);
    }

    #[test]
    fn correctly_decodes_legacy_pre_compatibility_justification() {
        let raw_justification_from_36322_testnet_block: Vec<u8> = vec![
            40, 1, 199, 1, 200, 78, 238, 115, 155, 39, 84, 168, 116, 176, 112, 170, 201, 173, 56,
            143, 238, 11, 77, 198, 165, 248, 213, 49, 128, 69, 26, 214, 67, 11, 244, 246, 157, 246,
            94, 26, 214, 225, 191, 14, 157, 33, 249, 41, 89, 82, 246, 84, 63, 79, 162, 54, 22, 113,
            2, 223, 211, 34, 70, 242, 203, 9, 0, 1, 98, 91, 163, 81, 213, 24, 239, 15, 16, 87, 135,
            254, 59, 111, 43, 10, 111, 246, 176, 92, 0, 36, 255, 92, 176, 245, 127, 211, 13, 226,
            66, 126, 181, 150, 136, 24, 29, 145, 178, 53, 87, 146, 87, 176, 37, 60, 100, 158, 147,
            120, 132, 58, 127, 30, 36, 241, 142, 134, 17, 196, 251, 65, 252, 8, 0, 1, 86, 161, 136,
            183, 233, 119, 120, 28, 171, 218, 36, 132, 125, 237, 163, 126, 31, 233, 216, 111, 72,
            120, 215, 46, 176, 205, 136, 80, 3, 219, 189, 49, 254, 0, 12, 31, 24, 199, 243, 99,
            165, 18, 78, 212, 163, 57, 58, 250, 87, 44, 247, 232, 84, 214, 15, 119, 108, 219, 74,
            27, 198, 203, 153, 0, 0, 1, 97, 2, 203, 187, 223, 134, 167, 54, 202, 165, 58, 72, 245,
            75, 53, 49, 68, 64, 183, 180, 54, 88, 17, 184, 200, 204, 25, 144, 75, 127, 67, 113,
            241, 142, 79, 183, 22, 151, 122, 12, 252, 230, 76, 81, 2, 18, 43, 44, 50, 170, 23, 224,
            161, 226, 136, 232, 83, 156, 214, 101, 129, 10, 173, 11, 1, 198, 232, 186, 138, 241,
            202, 17, 37, 2, 91, 115, 222, 138, 206, 245, 78, 172, 224, 220, 236, 130, 207, 174,
            190, 174, 126, 57, 112, 213, 13, 77, 193, 235, 154, 18, 218, 231, 235, 182, 198, 200,
            109, 218, 132, 238, 49, 183, 228, 94, 142, 234, 46, 61, 192, 94, 143, 129, 76, 160,
            126, 91, 159, 33, 8, 1, 184, 236, 59, 12, 141, 86, 72, 1, 76, 207, 155, 139, 118, 167,
            168, 2, 88, 40, 243, 29, 227, 103, 229, 221, 40, 156, 172, 114, 33, 47, 147, 44, 32,
            147, 94, 227, 205, 157, 116, 242, 24, 74, 151, 239, 141, 128, 70, 113, 165, 118, 251,
            98, 155, 9, 155, 69, 176, 2, 105, 227, 27, 46, 199, 10, 1, 32, 104, 60, 113, 219, 179,
            210, 191, 154, 5, 237, 128, 101, 82, 78, 216, 251, 232, 106, 133, 137, 245, 44, 106,
            186, 24, 31, 73, 98, 183, 24, 133, 102, 242, 134, 229, 149, 202, 102, 33, 3, 187, 126,
            249, 0, 104, 236, 225, 202, 93, 227, 57, 246, 97, 100, 0, 116, 162, 252, 224, 251, 57,
            0, 15,
        ];
        assert_backwards_compatible_decodes_pre_compatibility_justification(
            raw_justification_from_36322_testnet_block,
            10,
        );
    }
}
