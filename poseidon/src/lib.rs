#![cfg_attr(not(feature = "std"), no_std)]

/// Circuit field reexported from `ark_bls12_381`.
pub use ark_bls12_381::Fr;
/// Numeric type reexported from `ark_ff`.
pub use ark_ff::biginteger::BigInteger256;

/// Circuit counterparts to the methods in [hash] module. Available only under `circuit` feature.
#[cfg(feature = "circuit")]
pub mod circuit;
/// Hashing raw field elements.
pub mod hash;
mod parameters;

/// Poseidon paper suggests using domain separation for concretely encoding the use case in the
/// capacity element (which is fine as it is 256 bits large and has a lot of bits to fill).
pub const DOMAIN_SEPARATOR: u64 = 2137;

/// Return [DOMAIN_SEPARATOR] as a field element.
pub fn domain_separator() -> Fr {
    DOMAIN_SEPARATOR.into()
}
