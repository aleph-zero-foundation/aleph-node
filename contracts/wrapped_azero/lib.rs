#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![feature(min_specialization)]
#![allow(clippy::let_unit_value)]

pub use crate::wrapped_azero::{
    ALLOWANCE_SELECTOR, BALANCE_OF_SELECTOR, TRANSFER_FROM_SELECTOR, TRANSFER_SELECTOR,
};

#[openbrush::contract]
pub mod wrapped_azero {
    use ink::{
        codegen::{EmitEvent, Env},
        prelude::format,
        reflect::ContractEventBase,
    };
    use num_traits::identities::Zero;
    use openbrush::{
        contracts::psp22::{extensions::metadata::*, Internal, PSP22Error},
        traits::Storage,
    };

    pub const BALANCE_OF_SELECTOR: [u8; 4] = [0x65, 0x68, 0x38, 0x2f];
    pub const TRANSFER_SELECTOR: [u8; 4] = [0xdb, 0x20, 0xf9, 0xf5];
    pub const TRANSFER_FROM_SELECTOR: [u8; 4] = [0x54, 0xb3, 0xc7, 0x6e];
    pub const ALLOWANCE_SELECTOR: [u8; 4] = [0x4d, 0x47, 0xd9, 0x21];

    #[ink(storage)]
    #[derive(Storage)]
    pub struct WrappedAzero {
        #[storage_field]
        psp22: psp22::Data,
        #[storage_field]
        metadata: metadata::Data,
    }

    impl Default for WrappedAzero {
        fn default() -> Self {
            Self::new()
        }
    }

    impl PSP22 for WrappedAzero {}

    impl PSP22Metadata for WrappedAzero {}

    // emit events
    // https://github.com/w3f/PSPs/blob/master/PSPs/psp-22.md
    impl Internal for WrappedAzero {
        fn _emit_transfer_event(
            &self,
            _from: Option<AccountId>,
            _to: Option<AccountId>,
            _amount: Balance,
        ) {
            WrappedAzero::emit_event(
                self.env(),
                Event::Transfer(Transfer {
                    from: _from,
                    to: _to,
                    value: _amount,
                }),
            );
        }

        fn _emit_approval_event(&self, _owner: AccountId, _spender: AccountId, _amount: Balance) {
            WrappedAzero::emit_event(
                self.env(),
                Event::Approval(Approval {
                    owner: _owner,
                    spender: _spender,
                    value: _amount,
                }),
            );
        }
    }

    /// Result type
    pub type Result<T> = core::result::Result<T, PSP22Error>;
    /// Event type
    pub type Event = <WrappedAzero as ContractEventBase>::Type;

    /// Event emitted when a token transfer occurs.
    #[ink(event)]
    #[derive(Debug)]
    pub struct Transfer {
        #[ink(topic)]
        pub from: Option<AccountId>,
        #[ink(topic)]
        pub to: Option<AccountId>,
        pub value: Balance,
    }

    /// Event emitted when an approval occurs that `spender` is allowed to withdraw
    /// up to the amount of `value` tokens from `owner`.
    #[ink(event)]
    #[derive(Debug)]
    pub struct Approval {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        spender: AccountId,
        value: Balance,
    }

    #[ink(event)]
    #[derive(Debug)]
    pub struct Wrapped {
        #[ink(topic)]
        caller: AccountId,
        amount: Balance,
    }

    #[ink(event)]
    #[derive(Debug)]
    pub struct Unwrapped {
        #[ink(topic)]
        caller: AccountId,
        amount: Balance,
    }

    impl WrappedAzero {
        /// Creates a new token
        ///
        /// The token will have its name and symbol set in metadata to the specified values.
        /// Decimals are fixed at 12.
        ///
        /// Will revert if called from an account without a proper role
        #[ink(constructor)]
        pub fn new() -> Self {
            let metadata = metadata::Data {
                name: Some("wAzero".into()),
                symbol: Some("wA0".into()),
                decimals: 12, // same as AZERO
                ..Default::default()
            };

            Self {
                psp22: psp22::Data::default(),
                metadata,
            }
        }

        /// Terminates the contract.
        ///
        /// No-op by default, can only be compiled with a flag in dev environments
        #[ink(message)]
        pub fn terminate(&mut self) -> Result<()> {
            cfg_if::cfg_if! { if #[cfg( feature = "devnet" )] {
                let caller = self.env().caller();
                self.env().terminate_contract(caller)
            } else {
                panic!("this contract cannot be terminated")
            }}
        }

        /// Wraps the transferred amount of native token and mints it to the callers account
        #[ink(message, payable)]
        pub fn wrap(&mut self) -> Result<()> {
            let caller = self.env().caller();
            let amount = self.env().transferred_value();
            if !amount.eq(&Balance::zero()) {
                self._mint_to(caller, amount)?;
                Self::emit_event(self.env(), Event::Wrapped(Wrapped { caller, amount }));
            }

            Ok(())
        }

        /// Unwraps a specified amount
        #[ink(message)]
        pub fn unwrap(&mut self, amount: Balance) -> Result<()> {
            if amount.eq(&Balance::zero()) {
                return Ok(());
            }

            let caller = self.env().caller();

            // burn the token form the caller, will fail if the calling account doesn't have enough balance
            self._burn_from(caller, amount)?;

            // return the native token to the caller
            self.env().transfer(caller, amount).map_err(|why| {
                PSP22Error::Custom(format!("Native transfer failed: {:?}", why).into())
            })?;
            Self::emit_event(self.env(), Event::Unwrapped(Unwrapped { caller, amount }));

            Ok(())
        }

        /// Returns own code hash
        #[ink(message)]
        pub fn code_hash(&self) -> Result<Hash> {
            Self::env().own_code_hash().map_err(|why| {
                PSP22Error::Custom(format!("Can't retrieve own code hash: {:?}", why).into())
            })
        }

        pub fn emit_event<EE: EmitEvent<Self>>(emitter: EE, event: Event) {
            emitter.emit_event(event);
        }
    }
}
