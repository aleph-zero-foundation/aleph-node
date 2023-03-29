//! Module exposing some utilities regarding note generation and verification.

use ark_std::{vec, vec::Vec};
use liminal_ark_poseidon::hash;

use super::types::{
    FrontendNote, FrontendNullifier, FrontendTokenAmount, FrontendTokenId, FrontendTrapdoor,
};
use crate::{environment::CircuitField, shielder::convert_hash};

/// Compute note as the result of hashing `(token_id, token_amount, trapdoor, nullifier)`.
///
/// Useful for input preparation and offline note generation.
pub fn compute_note(
    token_id: FrontendTokenId,
    token_amount: FrontendTokenAmount,
    trapdoor: FrontendTrapdoor,
    nullifier: FrontendNullifier,
) -> FrontendNote {
    hash::four_to_one_hash([
        CircuitField::from(token_id as u64),
        CircuitField::from(token_amount),
        convert_hash(trapdoor),
        convert_hash(nullifier),
    ])
    .0
     .0
}

pub fn compute_parent_hash(left: FrontendNote, right: FrontendNote) -> FrontendNote {
    hash::two_to_one_hash([convert_hash(left), convert_hash(right)])
        .0
         .0
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
        let trapdoor: FrontendTrapdoor = [17; 4];
        let nullifier: FrontendNullifier = [19; 4];
        let note = compute_note(token_id, token_amount, trapdoor, nullifier);

        let bytes = bytes_from_note(&note);
        let note_again = note_from_bytes(&bytes);

        assert_eq!(note, note_again);
    }
}
