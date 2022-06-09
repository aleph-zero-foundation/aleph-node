use crate::{
    crypto::{Signature, SignatureV1},
    justification::AlephJustification,
};
use aleph_bft::{PartialMultisignature, SignatureSet};
use codec::{Decode, DecodeAll, Encode};

/// Old format of justifications, needed for backwards compatibility.
#[derive(Clone, Encode, Decode, Debug, PartialEq, Eq)]
pub struct AlephJustificationV1 {
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
pub enum JustificationDecoding {
    V1(AlephJustificationV1),
    V2(AlephJustification),
    Err,
}

pub fn backwards_compatible_decode(justification_raw: Vec<u8>) -> JustificationDecoding {
    let justification_cloned = justification_raw.clone();
    if let Ok(justification) = AlephJustification::decode_all(&mut justification_cloned.as_slice())
    {
        JustificationDecoding::V2(justification)
    } else if let Ok(justification) =
        AlephJustificationV1::decode_all(&mut justification_raw.as_slice())
    {
        JustificationDecoding::V1(justification)
    } else {
        JustificationDecoding::Err
    }
}
