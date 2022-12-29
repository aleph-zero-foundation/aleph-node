#![cfg_attr(not(feature = "std"), no_std)]

//! This is a simple example contract for use with e2e tests of the aleph-client contract interaction.

#[ink::contract]
mod adder {
    #[ink(storage)]
    pub struct Adder {
        name: Option<[u8; 20]>,
        value: u32,
    }

    #[ink(event)]
    pub struct ValueChanged {
        new_value: u32,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Overflow,
    }

    impl Adder {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                value: 0,
                name: None,
            }
        }

        #[ink(message)]
        pub fn add(&mut self, value: u32) -> Result<(), Error> {
            self.value = self.value.checked_add(value).ok_or(Error::Overflow)?;

            Self::env().emit_event(ValueChanged {
                new_value: self.value,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn get(&self) -> u32 {
            self.value
        }

        #[ink(message)]
        pub fn set_name(&mut self, name: Option<[u8; 20]>) {
            self.name = name;
        }

        #[ink(message)]
        pub fn get_name(&self) -> Option<[u8; 20]> {
            self.name
        }
    }

    impl Default for Adder {
        fn default() -> Self {
            Self::new()
        }
    }
}
