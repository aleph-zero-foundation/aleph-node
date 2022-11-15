use ink_prelude::{format, string::String};
use openbrush::contracts::psp22::PSP22Error;
use scale::{Decode, Encode};
use snarcos_extension::SnarcosError;

#[derive(Eq, PartialEq, Debug, Decode, Encode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum BlenderError {
    /// Caller is missing some permission.
    InsufficientPermission,
    /// Merkle tree is full - no new notes can be created.
    TooManyNotes,

    /// Pallet returned an error (through chain extension).
    ChainExtension(SnarcosError),

    /// PSP22 related error (e.g. insufficient allowance).
    Psp22(PSP22Error),
    /// Environment error (e.g. non-existing token contract).
    InkEnv(String),

    /// This token id is already taken.
    TokenIdAlreadyRegistered,
    /// There is no registered token under this token id.
    TokenIdNotRegistered,
}

impl From<SnarcosError> for BlenderError {
    fn from(e: SnarcosError) -> Self {
        BlenderError::ChainExtension(e)
    }
}

impl From<PSP22Error> for BlenderError {
    fn from(e: PSP22Error) -> Self {
        BlenderError::Psp22(e)
    }
}

impl From<ink_env::Error> for BlenderError {
    fn from(e: ink_env::Error) -> Self {
        BlenderError::InkEnv(format!("{:?}", e))
    }
}
