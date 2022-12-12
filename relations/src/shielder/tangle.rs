//! This module provides 'tangling' - some cheap substitute for real hash function.
//!
//! Tangling is a function that takes in a sequence of bytes (either raw bytes (`tangle`) or as
//! field bytes gadgets (`tangle_in_field`)) and manipulates it. It operates in two steps, as
//! follows:
//!  1.1 For every chunk of length `BASE_LENGTH` we compute suffix sums.
//!  1.2 We build a binary tree over these chunks.
//!  1.3 We go bottom-to-top and in every intermediate node we:
//!      1.3.1 swap the halves
//!      1.3.2 compute prefix products
//!  2.1 Given a new mangled sequence of `n` elements we squash it `SQUASH_FACTOR` times, i.e. we
//!      take chunks of length `SQUASH_FACTOR` and reduce them to a single byte by xoring.
//!
//! Note, it is **not** hiding like any hashing function.
//!
//! This module exposes two implementations of tangling: `tangle` and `tangle_in_field`. They are
//! semantically equivalent, but they just operate on different element types.
//!
//! All the index intervals used here are closed-open, i.e. they are in form `[a, b)`, which means
//! that we consider indices `a`, `a+1`, ..., `b-1`. We also use 0-based indexing.

use ark_r1cs_std::R1CSVar;
use ark_relations::r1cs::SynthesisError;

use super::types::ByteVar;

/// Bottom-level chunk length.
const BASE_LENGTH: usize = 4;

/// Tangle elements of `bytes`.
///
/// For circuit use only.
pub(super) fn tangle_in_field<const SQUASH_FACTOR: usize>(
    mut bytes: Vec<ByteVar>,
) -> Result<Vec<ByteVar>, SynthesisError> {
    let number_of_bytes = bytes.len();
    _tangle_in_field(&mut bytes, 0, number_of_bytes)?;
    Ok(bytes
        .chunks(SQUASH_FACTOR)
        .map(|chunk| {
            chunk
                .iter()
                .cloned()
                .reduce(|x, y| x.xor(&y).unwrap())
                .unwrap()
        })
        .collect())
}

/// Recursive and index-bounded implementation of the first step of the `tangle` procedure.
fn _tangle_in_field(bytes: &mut [ByteVar], low: usize, high: usize) -> Result<(), SynthesisError> {
    // Bottom level case: computing suffix sums. We have to do some loop-index boilerplate, because
    // Rust doesn't support decreasing range iteration.
    if high - low <= BASE_LENGTH {
        let mut i = high - 2;
        loop {
            bytes[i] = ByteVar::constant(
                u8::overflowing_add(
                    bytes[i].value().unwrap_or_default(),
                    bytes[i + 1].value().unwrap_or_default(),
                )
                .0,
            );
            if i == low {
                break;
            } else {
                i -= 1
            }
        }
    } else {
        // We are in some inner node of the virtual binary tree.
        //
        // We start by recursive call to both halves, so that we proceed in a bottom-top manner.
        let mid = (low + high) / 2;
        _tangle_in_field(bytes, low, mid)?;
        _tangle_in_field(bytes, mid, high)?;

        // Swapping the halves.
        for i in low..mid {
            let temp = bytes[i].clone();
            bytes[i] = bytes[i + mid - low].clone();
            bytes[i + mid - low] = temp;
        }

        // Prefix products.
        for i in low + 1..high {
            bytes[i] = ByteVar::constant(
                u8::overflowing_mul(
                    bytes[i].value().unwrap_or_default(),
                    bytes[i - 1].value().unwrap_or_default(),
                )
                .0,
            )
        }
    }
    Ok(())
}

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
/// For detailed description, see `_tangle_in_field`.
fn _tangle(bytes: &mut [u8], low: usize, high: usize) {
    if high - low <= BASE_LENGTH {
        let mut i = high - 2;
        loop {
            bytes[i] = u8::overflowing_add(bytes[i], bytes[i + 1]).0;
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
            bytes[i] = u8::overflowing_mul(bytes[i], bytes[i - 1]).0;
        }
    }
}
