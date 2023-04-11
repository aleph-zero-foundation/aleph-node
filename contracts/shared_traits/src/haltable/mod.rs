#![allow(clippy::inline_fn_without_body)]
use ink::prelude::{format, string::String};
use openbrush::{
    contracts::psp22::PSP22Error,
    traits::{Storage, StorageAsMut, StorageAsRef},
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
            <Self as StorageAsMut>::data(self).halted = true;
            self._after_halt()?;
        }
        Ok(())
    }

    default fn resume(&mut self) -> HaltableResult<()> {
        self._before_resume()?;
        if self.is_halted() {
            <Self as StorageAsMut>::data(self).halted = false;
            self._after_resume()?;
        }
        Ok(())
    }

    default fn is_halted(&self) -> bool {
        <Self as StorageAsRef>::data(self).halted
    }

    default fn check_halted(&self) -> HaltableResult<()> {
        match self.is_halted() {
            true => Err(HaltableError::InHaltedState),
            false => Ok(()),
        }
    }
}

pub const STORAGE_KEY: u32 = openbrush::storage_unique_key!(HaltableData);

#[derive(Debug)]
#[openbrush::upgradeable_storage(STORAGE_KEY)]
pub struct HaltableData {
    pub halted: bool,
}
