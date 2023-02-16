#![cfg_attr(not(feature = "std"), no_std)]

use ark_bls12_381::Fr;

#[cfg(feature = "circuit")]
pub mod circuit;
pub mod hash;
mod parameters;

const DOMAIN_SEPARATOR: u64 = 2137;

/// Poseidon paper suggests using domain separation for concretely encoding the use case in the
/// capacity element (which is fine as it is 256 bits large and has a lot of bits to fill).
pub fn domain_separator() -> Fr {
    DOMAIN_SEPARATOR.into()
}
