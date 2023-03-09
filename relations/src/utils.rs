use ark_ff::{One, Zero};
use ark_std::{string::String, vec::Vec};

/// Convert `u8` into an 8-tuple of bits over `F` (little endian).
pub fn byte_to_bits<F: Zero + One + Copy>(byte: &u8) -> [F; 8] {
    let mut bits = [F::zero(); 8];
    for (idx, bit) in bits.iter_mut().enumerate() {
        if (byte >> idx) & 1 == 1 {
            *bit = F::one();
        }
    }
    bits
}

/// Takes a string an converts it to a 32 byte array
/// missing bytes are padded with 0's
pub fn string_to_padded_bytes(s: String) -> [u8; 32] {
    let mut bytes: Vec<u8> = s.as_bytes().into();
    bytes.resize(32, 0);
    bytes.try_into().expect("this should never fail")
}
