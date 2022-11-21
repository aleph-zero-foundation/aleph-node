//! This module is only temporary and should be substituted by a dependency to a small utility crate
//! from `HouseSnark` repository.
//!
//! All these functions are just copied from `HouseSnark`.

use ark_ff::{BigInteger, BigInteger256};
use ink_prelude::vec::Vec;

use crate::Note;

/// Bottom-level chunk length.
const BASE_LENGTH: usize = 4;

/// Tangle elements of `bytes`.
pub fn tangle<const SQUASH_FACTOR: usize>(mut bytes: Vec<u8>) -> Vec<u8> {
    let number_of_bytes = bytes.len();
    _tangle(&mut bytes, 0, number_of_bytes);
    bytes
        .chunks(SQUASH_FACTOR)
        .map(|chunk| chunk.iter().cloned().reduce(|x, y| x ^ y).unwrap())
        .collect()
}

/// Recursive and index-bounded implementation of the first step of the `tangle` procedure.
///
/// For detailed description, consult `HouseSnark` repository.
fn _tangle(bytes: &mut [u8], low: usize, high: usize) {
    if high - low <= BASE_LENGTH {
        let mut i = high - 2;
        loop {
            bytes[i] += bytes[i + 1];
            if i == low {
                break;
            } else {
                i -= 1
            }
        }
    } else {
        let mid = (low + high) / 2;
        _tangle(bytes, low, mid);
        _tangle(bytes, mid, high);

        for i in low..mid {
            bytes.swap(i, i + mid - low);
        }

        for i in low + 1..high {
            bytes[i] *= bytes[i - 1]
        }
    }
}

/// Compute hash in the parent node, given its left and right child;
pub(super) fn compute_parent_hash(left: &Note, right: &Note) -> Note {
    let bytes = [
        BigInteger256::new(*left).to_bytes_le(),
        BigInteger256::new(*right).to_bytes_le(),
    ]
    .concat();
    note_from_bytes(tangle::<2>(bytes).as_slice())
}

/// Create a note from the first 32 bytes of `bytes`.
fn note_from_bytes(bytes: &[u8]) -> Note {
    [
        u64::from_le_bytes(bytes[0..8].try_into().unwrap()),
        u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
        u64::from_le_bytes(bytes[16..24].try_into().unwrap()),
        u64::from_le_bytes(bytes[24..32].try_into().unwrap()),
    ]
}
