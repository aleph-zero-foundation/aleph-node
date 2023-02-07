//! This module provides 'tangling' - some cheap substitute for real hash function.
//!
//! Tangling is a function that takes in a sequence of field elements (either raw `CircuitField`s
//! ([tangle]) or as `FpVar`s ([tangle_in_circuit])) and mixes it into a single field element.
//!
//! It operates in three steps:
//!  1.1 We repeat the sequence until the result has [EXPAND_TO] elements. If the input sequence is
//!      longer than [EXPAND_TO], it will be trimmed.
//!  2.1 For every chunk of length `BASE_LENGTH` we compute _spiced_ suffix sums of inverses.
//!     Basically, apart from just summing element inverses, we multiply intermediate results by an
//!     index-dependent factor.
//!  2.2 We build a binary tree over these chunks.
//!  2.3 We go bottom-to-top and in every intermediate node we:
//!      1.3.1 swap the halves
//!      1.3.2 compute prefix products
//!  3.1 A new mangled sequence of `n` elements is reduced by summing.
//!
//! In some places, where an element turns out to be zero, we replace it by an index-dependent
//! constant.
//!
//! Note, it is **not** hiding like any hashing function.
//!
//! This module exposes two implementations of tangling: [tangle] and [tangle_in_circuit]. They are
//! semantically equivalent, but they just operate on different element types.
//!
//! All the index intervals used here are closed-open, i.e. they are in form `[a, b)`, which means
//! that we consider indices `a`, `a+1`, ..., `b-1`. We also use 0-based indexing.

use core::ops::Add;

use ark_ff::{Field, Zero};
use ark_r1cs_std::fields::FieldVar;
use ark_relations::r1cs::SynthesisError;
use ark_std::vec::Vec;

use crate::{environment::FpVar, CircuitField};

/// Bottom-level chunk length.
const BASE_LENGTH: usize = 4;
/// Tangling operates on fixed-sized arrays with 128 elements.
const EXPAND_TO: usize = 128;

/// Entangle `input` into single `FpVar`.
///
/// For circuit use only.
pub(super) fn tangle_in_circuit(input: &[FpVar]) -> Result<FpVar, SynthesisError> {
    let mut input_expanded = input
        .iter()
        .cycle()
        .take(EXPAND_TO)
        .cloned()
        .collect::<Vec<_>>();

    do_tangle_in_circuit(&mut input_expanded, 0, EXPAND_TO)?;

    Ok(input_expanded.into_iter().reduce(|a, b| a.add(b)).unwrap())
}

fn dezeroize_in_circuit(
    fp: &FpVar,
    fallback: impl Into<CircuitField>,
) -> Result<FpVar, SynthesisError> {
    let fallback = FpVar::constant(fallback.into());
    fp.is_zero()?.select(&fallback, fp)
}

/// Recursive and index-bounded implementation of the first step of the `tangle` procedure.
fn do_tangle_in_circuit(
    elems: &mut [FpVar],
    low: usize,
    high: usize,
) -> Result<(), SynthesisError> {
    // Bottom level case: computing suffix sums of inverses. We have to do some loop-index
    // boilerplate, because Rust doesn't support decreasing range iteration.
    if high - low <= BASE_LENGTH {
        let mut i = high - 2;
        loop {
            let previous = dezeroize_in_circuit(&elems[i + 1], (2 * low + 1) as u64)?;
            let current = dezeroize_in_circuit(&elems[i], (2 * high + 1) as u64)?;

            elems[i] =
                (previous + current.inverse()?) * FpVar::constant(CircuitField::from(i as u64));

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
        do_tangle_in_circuit(elems, low, mid)?;
        do_tangle_in_circuit(elems, mid, high)?;

        // Swapping the halves.
        for i in low..mid {
            elems.swap(i, i + mid - low);
        }

        // Prefix products.
        for i in low + 1..high {
            let product = &elems[i] * &elems[i - 1];
            elems[i] = dezeroize_in_circuit(&product, (low * high + i) as u64)?;
        }
    }
    Ok(())
}

/// Tangle elements of `bytes`.
pub fn tangle(input: &[CircuitField]) -> CircuitField {
    let mut input_expanded = input
        .iter()
        .cycle()
        .take(EXPAND_TO)
        .cloned()
        .collect::<Vec<_>>();

    do_tangle(&mut input_expanded, 0, EXPAND_TO);

    input_expanded.into_iter().sum()
}

fn dezeroize(fp: &CircuitField, fallback: impl Into<CircuitField>) -> CircuitField {
    if fp.is_zero() {
        fallback.into()
    } else {
        *fp
    }
}

/// Recursive and index-bounded implementation of the first step of the `tangle` procedure.
///
/// For detailed description, see [do_tangle_in_circuit].
fn do_tangle(elems: &mut [CircuitField], low: usize, high: usize) {
    if high - low <= BASE_LENGTH {
        let mut i = high - 2;
        loop {
            let previous = dezeroize(&elems[i + 1], (2 * low + 1) as u64);
            let current = dezeroize(&elems[i], (2 * high + 1) as u64);
            elems[i] = (previous + current.inverse().expect("Inverse of non-zero exists"))
                * CircuitField::from(i as u64);

            if i == low {
                break;
            } else {
                i -= 1
            }
        }
    } else {
        let mid = (low + high) / 2;
        do_tangle(elems, low, mid);
        do_tangle(elems, mid, high);

        for i in low..mid {
            elems.swap(i, i + mid - low);
        }

        for i in low + 1..high {
            let product = elems[i] * elems[i - 1];
            elems[i] = dezeroize(&product, (low * high + i) as u64);
        }
    }
}

#[cfg(test)]
mod tests {
    use ark_ff::Zero;
    use ark_r1cs_std::{fields::FieldVar, R1CSVar};

    use crate::{
        environment::FpVar,
        shielder::tangle::{tangle, tangle_in_circuit},
        CircuitField,
    };

    #[test]
    fn tangling_is_homomorphic() {
        let input = vec![
            CircuitField::from(0u64),
            CircuitField::from(100u64),
            CircuitField::from(17u64),
            CircuitField::from(19u64),
        ];
        let tangled = tangle(&input);

        let input_in_circuit = input.into_iter().map(FpVar::constant).collect::<Vec<_>>();
        let tangled_in_circuit = tangle_in_circuit(&input_in_circuit).unwrap();

        assert_eq!(tangled, tangled_in_circuit.value().unwrap());
    }

    #[test]
    fn tangles_zeros_to_non_zero() {
        let input = vec![CircuitField::zero(); 32];

        let tangled = tangle(&input);
        assert!(!tangled.0 .0.into_iter().any(|b| b.is_zero()));
    }

    #[test]
    fn tangles_small_values_to_non_zero() {
        let input = vec![
            CircuitField::from(41),
            CircuitField::from(314),
            CircuitField::from(1729),
        ];

        let tangled = tangle(&input);
        assert!(!tangled.0 .0.into_iter().any(|b| b.is_zero()));
    }
}
