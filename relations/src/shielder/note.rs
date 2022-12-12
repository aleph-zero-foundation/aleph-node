//! Module exposing some utilities regarding note generation and verification.

use ark_ff::{BigInteger, BigInteger256};
use ark_r1cs_std::{eq::EqGadget, ToBytesGadget};
use ark_relations::r1cs::SynthesisError;

use super::{
    tangle::{tangle, tangle_in_field},
    types::{
        ByteVar, FpVar, FrontendNote, FrontendNullifier, FrontendTokenAmount, FrontendTokenId,
        FrontendTrapdoor,
    },
};

/// Verify that `note` is indeed the result of tangling `(token_id, token_amount, trapdoor,
/// nullifier)`.
///
/// For circuit use only.
pub(super) fn check_note(
    token_id: &FpVar,
    token_amount: &FpVar,
    trapdoor: &FpVar,
    nullifier: &FpVar,
    note: &FpVar,
) -> Result<(), SynthesisError> {
    let bytes: Vec<ByteVar> = [
        token_id.to_bytes()?,
        token_amount.to_bytes()?,
        trapdoor.to_bytes()?,
        nullifier.to_bytes()?,
    ]
    .concat();
    let bytes = tangle_in_field::<4>(bytes)?;

    for (a, b) in note.to_bytes()?.iter().zip(bytes.iter()) {
        a.enforce_equal(b)?;
    }
    Ok(())
}

/// Compute note as the result of tangling `(token_id, token_amount, trapdoor, nullifier)`.
///
/// Useful for input preparation and offline note generation.
pub fn compute_note(
    token_id: FrontendTokenId,
    token_amount: FrontendTokenAmount,
    trapdoor: FrontendTrapdoor,
    nullifier: FrontendNullifier,
) -> FrontendNote {
    let bytes = [
        BigInteger256::from(token_id as u64).to_bytes_le(),
        BigInteger256::from(token_amount).to_bytes_le(),
        BigInteger256::from(trapdoor).to_bytes_le(),
        BigInteger256::from(nullifier).to_bytes_le(),
    ]
    .concat();

    note_from_bytes(tangle::<4>(bytes).as_slice())
}

pub fn compute_parent_hash(left: FrontendNote, right: FrontendNote) -> FrontendNote {
    let bytes = [
        BigInteger256::new(left).to_bytes_le(),
        BigInteger256::new(right).to_bytes_le(),
    ]
    .concat();
    note_from_bytes(tangle::<2>(bytes).as_slice())
}

/// Create a note from the first 32 bytes of `bytes`.
pub fn note_from_bytes(bytes: &[u8]) -> FrontendNote {
    [
        u64::from_le_bytes(bytes[0..8].try_into().unwrap()),
        u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
        u64::from_le_bytes(bytes[16..24].try_into().unwrap()),
        u64::from_le_bytes(bytes[24..32].try_into().unwrap()),
    ]
}

fn convert(x: u64) -> [u8; 8] {
    x.to_le_bytes()
}

pub fn bytes_from_note(note: &FrontendNote) -> Vec<u8> {
    let mut res = vec![];
    for elem in note {
        let mut arr: Vec<u8> = convert(*elem).into();
        res.append(&mut arr);
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_conversion() {
        let token_id: FrontendTokenId = 1;
        let token_amount: FrontendTokenAmount = 10;
        let trapdoor: FrontendTrapdoor = 17;
        let nullifier: FrontendNullifier = 19;
        let note = compute_note(token_id, token_amount, trapdoor, nullifier);

        let bytes = bytes_from_note(&note);
        let note_again = note_from_bytes(&bytes);

        assert_eq!(note, note_again);
    }
}
