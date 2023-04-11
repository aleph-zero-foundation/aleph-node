use access_control::roles::Role;
use ink::{
    env::Error as InkEnvError,
    prelude::{format, string::String},
    LangError,
};
use marketplace::marketplace::Error as MarketplaceError;
use openbrush::contracts::psp22::PSP22Error;
use shared_traits::HaltableError;

/// GameError types
#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum GameError {
    /// Wrapper for Haltable errors
    HaltableError(HaltableError),
    /// Reset has been called before the deadline
    BeforeDeadline,
    /// Button has been pressed after the deadline
    AfterDeadline,
    /// Call has been made from an account with missing access control privileges
    MissingRole(Role),
    /// A call to a PSP22 contract has failed
    PSP22Error(PSP22Error),
    /// An interaction with ink! environment has failed
    InkEnvError(String),
    /// Couldn't have retrieved own code hash
    CantRetrieveOwnCodeHash,
    /// Overflow error
    Arithmethic,
    /// Error from the marketplace contract
    MarketplaceError(MarketplaceError),
    /// Error while calling another contract
    ContractCall(LangError),
}

impl From<PSP22Error> for GameError {
    fn from(e: PSP22Error) -> Self {
        GameError::PSP22Error(e)
    }
}

impl From<InkEnvError> for GameError {
    fn from(e: InkEnvError) -> Self {
        GameError::InkEnvError(format!("{:?}", e))
    }
}

impl From<MarketplaceError> for GameError {
    fn from(e: MarketplaceError) -> Self {
        GameError::MarketplaceError(e)
    }
}

impl From<LangError> for GameError {
    fn from(e: LangError) -> Self {
        GameError::ContractCall(e)
    }
}

impl From<GameError> for HaltableError {
    fn from(why: GameError) -> Self {
        HaltableError::Custom(format!("{:?}", why))
    }
}

impl From<HaltableError> for GameError {
    fn from(inner: HaltableError) -> Self {
        GameError::HaltableError(inner)
    }
}
