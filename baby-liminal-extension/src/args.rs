//! Module gathering all the chain extension arguments. They can be used in the smart contract for a
//! proper argument encoding. On the runtime side, they can be used for decoding the arguments.

#[cfg(feature = "ink")]
use ink::prelude::vec::Vec;
#[cfg(feature = "runtime")]
use {
    parity_scale_codec::{Decode, Encode},
    sp_std::vec::Vec,
};

/// A struct describing layout for the `verify` chain extension.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ink", ink::scale_derive(Encode, Decode))]
#[cfg_attr(feature = "runtime", derive(Encode, Decode))]
pub struct VerifyArgs {
    /// The hash of the verification key.
    pub verification_key_hash: crate::KeyHash,
    /// The proof.
    pub proof: Vec<u8>,
    /// The public input.
    pub public_input: Vec<u8>,
}
