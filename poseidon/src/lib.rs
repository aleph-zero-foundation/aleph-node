#![cfg_attr(not(feature = "std"), no_std)]

use ark_bls12_381::Fr;
use once_cell::sync::Lazy;

pub mod circuit;
pub mod hash;
mod parameters;

/// Poseidon paper suggests using domain separation for concretely encoding the use case in the
/// capacity element (which is fine as it is 256 bits large and has a lot of bits to fill).
pub static DOMAIN_SEPARATOR: Lazy<Fr> = Lazy::new(|| Fr::from(2137));
