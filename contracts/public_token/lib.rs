#![cfg_attr(not(feature = "std"), no_std)]
#![feature(min_specialization)]

/// Most basic PSP22 token.
#[openbrush::contract]
#[allow(clippy::let_unit_value)] // Clippy shouts about returning anything from messages.
pub mod my_psp22 {
    use ink_storage::traits::SpreadAllocate;
    use openbrush::{contracts::psp22::*, traits::Storage};

    #[ink(storage)]
    #[derive(Default, SpreadAllocate, Storage)]
    pub struct Contract {
        #[storage_field]
        psp22: Data,
    }

    impl PSP22 for Contract {}

    impl Contract {
        /// Instantiate the contract with `total_supply` tokens of supply.
        ///
        /// All the created tokens will be minted to the caller.
        #[ink(constructor)]
        pub fn new(total_supply: Balance) -> Self {
            ink_lang::codegen::initialize_contract(|instance: &mut Contract| {
                instance
                    ._mint_to(instance.env().caller(), total_supply)
                    .expect("Should mint");
            })
        }
    }
}
