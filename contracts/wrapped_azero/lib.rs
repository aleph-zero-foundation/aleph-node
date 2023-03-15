#![cfg_attr(not(feature = "std"), no_std)]
#![feature(min_specialization)]
#![allow(clippy::let_unit_value)]

pub use crate::wrapped_azero::{
    ALLOWANCE_SELECTOR, BALANCE_OF_SELECTOR, TRANSFER_FROM_SELECTOR, TRANSFER_SELECTOR,
};

#[openbrush::contract]
pub mod wrapped_azero {
    use access_control::{roles::Role, AccessControlRef, ACCESS_CONTROL_PUBKEY};
    use ink::{
        codegen::{EmitEvent, Env},
        env::call::FromAccountId,
        prelude::format,
        reflect::ContractEventBase,
        ToAccountId,
    };
    use num_traits::identities::Zero;
    use openbrush::{
        contracts::psp22::{extensions::metadata::*, Internal},
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
        access_control: AccessControlRef,
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
        caller: AccountId,
        amount: Balance,
    }

    #[ink(event)]
    #[derive(Debug)]
    pub struct UnWrapped {
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
            let caller = Self::env().caller();
            let code_hash = Self::env()
                .own_code_hash()
                .expect("Called new on a contract with no code hash");

            let access_control = AccountId::from(ACCESS_CONTROL_PUBKEY);
            let access_control = AccessControlRef::from_account_id(access_control);
            if access_control.has_role(caller, Role::Initializer(code_hash)) {
                let metadata = metadata::Data {
                    name: Some("wAzero".into()),
                    symbol: Some("wA0".into()),
                    decimals: 12, // same as AZERO
                    ..Default::default()
                };

                Self {
                    psp22: psp22::Data::default(),
                    metadata,
                    access_control,
                }
            } else {
                panic!("Caller is not allowed to initialize this contract");
            }
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
            Self::emit_event(self.env(), Event::UnWrapped(UnWrapped { caller, amount }));

            Ok(())
        }

        /// Terminates the contract.
        ///
        /// can only be called by the contract's Owner
        #[ink(message)]
        pub fn terminate(&mut self) -> Result<()> {
            let caller = self.env().caller();
            let this = self.env().account_id();
            let required_role = Role::Owner(this);

            self.check_role(caller, required_role)?;
            self.env().terminate_contract(caller)
        }

        /// Returns the contract's access control contract address
        #[ink(message)]
        pub fn access_control(&self) -> AccountId {
            self.access_control.to_account_id()
        }

        /// Sets new access control contract address
        ///
        /// Can only be called by the contract's Owner
        #[ink(message)]
        pub fn set_access_control(&mut self, access_control: AccountId) -> Result<()> {
            let caller = self.env().caller();
            let this = self.env().account_id();

            self.check_role(caller, Role::Owner(this))?;

            self.access_control = AccessControlRef::from_account_id(access_control);
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

        fn check_role(&self, account: AccountId, role: Role) -> Result<()> {
            if self.access_control.has_role(account, role) {
                Ok(())
            } else {
                Err(PSP22Error::Custom(format!("MissingRole:{:?}", role).into()))
            }
        }
    }
}
