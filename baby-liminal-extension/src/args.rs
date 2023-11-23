//! Module gathering all the chain extension arguments. They can be used in the smart contract for a
//! proper argument encoding. On the runtime side, they can be used for decoding the arguments.

#[cfg(feature = "ink")]
use {crate::VerificationKeyIdentifier, ink::prelude::vec::Vec, ink::primitives::AccountId};
#[cfg(feature = "runtime")]
use {
    frame_support::sp_runtime::AccountId32 as AccountId,
    pallet_baby_liminal::VerificationKeyIdentifier, sp_std::vec::Vec,
};

/// A struct describing layout for the `store_key` chain extension.
#[derive(Clone, Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
pub struct StoreKeyArgs {
    /// The account ID of the key depositor.
    pub depositor: AccountId,
    /// The identifier of the key.
    pub identifier: VerificationKeyIdentifier,
    /// The key itself.
    pub key: Vec<u8>,
}

/// A struct describing layout for the `verify` chain extension.
#[derive(Clone, Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
pub struct VerifyArgs {
    /// The identifier of the verification key.
    pub verification_key_identifier: VerificationKeyIdentifier,
    /// The proof.
    pub proof: Vec<u8>,
    /// The public input.
    pub public_input: Vec<u8>,
}
