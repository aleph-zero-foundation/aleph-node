#![allow(clippy::inline_fn_without_body)]
use ink::{
    prelude::{format, string::String},
    storage::{traits::ManualKey, Lazy},
};
use openbrush::{
    contracts::psp22::PSP22Error,
    traits::{Storage, StorageAsMut},
};

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum HaltableError {
    InHaltedState,
    NotInHaltedState,
    Custom(String),
}

/// Result type
pub type HaltableResult<T> = Result<T, HaltableError>;

impl From<PSP22Error> for HaltableError {
    fn from(why: PSP22Error) -> Self {
        HaltableError::Custom(format!("{:?}", why))
    }
}

#[openbrush::trait_definition]
pub trait Haltable {
    #[ink(message)]
    fn halt(&mut self) -> HaltableResult<()>;

    #[ink(message)]
    fn resume(&mut self) -> HaltableResult<()>;

    #[ink(message)]
    fn is_halted(&self) -> bool;

    #[ink(message)]
    fn check_halted(&self) -> HaltableResult<()>;
}

pub trait Internal {
    fn _after_halt(&self) -> HaltableResult<()> {
        Ok(())
    }

    fn _after_resume(&self) -> HaltableResult<()> {
        Ok(())
    }

    fn _before_halt(&self) -> HaltableResult<()> {
        Ok(())
    }

    fn _before_resume(&self) -> HaltableResult<()> {
        Ok(())
    }
}

impl<T: Storage<HaltableData> + Internal> Haltable for T {
    default fn halt(&mut self) -> HaltableResult<()> {
        self._before_halt()?;
        if !self.is_halted() {
            // <Self as StorageAsMut>::data(self).halted = true;
            <Self as StorageAsMut>::data(self).halted.set(&true);
            self._after_halt()?;
        }
        Ok(())
    }

    default fn resume(&mut self) -> HaltableResult<()> {
        self._before_resume()?;
        if self.is_halted() {
            <Self as StorageAsMut>::data(self).halted.set(&false);
            self._after_resume()?;
        }
        Ok(())
    }

    default fn is_halted(&self) -> bool {
        self.data().halted.get_or_default()
    }

    default fn check_halted(&self) -> HaltableResult<()> {
        match self.is_halted() {
            true => Err(HaltableError::InHaltedState),
            false => Ok(()),
        }
    }
}

// NOTE: this is the REAL storage cell
pub const STORAGE_KEY: u32 = 0x48414C54;

#[derive(Debug)]
// NOTE: OB macro does not work as per the documentation.
// Whatever key you specify the data ends up as part of the default root key,
// therefore we do not bother specifying anything else here
// but rather wrap the underlying type as a Lazy storage cell.
#[openbrush::upgradeable_storage(0x00000000)]
pub struct HaltableData {
    pub halted: Lazy<bool, ManualKey<STORAGE_KEY>>,
}
