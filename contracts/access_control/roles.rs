use scale::{Decode, Encode};

use crate::{AccountId, Hash};

#[derive(Debug, Encode, Decode, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Role {
    /// Indicates account that can initialize a contract from a given code hash.
    Initializer(Hash),
    /// Indicates a superuser.
    ///
    /// Superuser can perform many potentialy destructive actions e.g. terminate a contract.
    Admin(AccountId),
    /// Indicates a custom role with a 4 byte identifier that has application-specific semantics.
    Custom(AccountId, [u8; 4]),
}
