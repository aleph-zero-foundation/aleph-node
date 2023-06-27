#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![feature(min_specialization)]
#![allow(clippy::let_unit_value)]

pub use crate::ticket_token::{BALANCE_OF_SELECTOR, TRANSFER_FROM_SELECTOR, TRANSFER_SELECTOR};

#[openbrush::contract]
pub mod ticket_token {
    use access_control::{roles::Role, AccessControlRef, ACCESS_CONTROL_PUBKEY};
    use ink::{
        codegen::{EmitEvent, Env},
        env::{
            call::{build_call, ExecutionInput, FromAccountId},
            set_code_hash, DefaultEnvironment,
        },
        prelude::{format, string::String},
        reflect::ContractEventBase,
        ToAccountId,
    };
    use openbrush::{
        contracts::psp22::{extensions::metadata::*, Internal},
        traits::Storage,
    };
    use shared_traits::Selector;

    pub const BALANCE_OF_SELECTOR: [u8; 4] = [0x65, 0x68, 0x38, 0x2f];
    pub const TRANSFER_SELECTOR: [u8; 4] = [0xdb, 0x20, 0xf9, 0xf5];
    pub const TRANSFER_FROM_SELECTOR: [u8; 4] = [0x54, 0xb3, 0xc7, 0x6e];

    #[ink(storage)]
    #[derive(Storage)]
    pub struct TicketToken {
        #[storage_field]
        psp22: psp22::Data,
        #[storage_field]
        metadata: metadata::Data,
        access_control: AccessControlRef,
    }

    impl PSP22 for TicketToken {}

    impl PSP22Metadata for TicketToken {}

    impl Internal for TicketToken {
        fn _emit_transfer_event(
            &self,
            from: Option<AccountId>,
            to: Option<AccountId>,
            amount: Balance,
        ) {
            TicketToken::emit_event(
                self.env(),
                Event::TransferEvent(TransferEvent {
                    from,
                    to,
                    value: amount,
                }),
            );
        }

        fn _emit_approval_event(&self, owner: AccountId, spender: AccountId, amount: Balance) {
            TicketToken::emit_event(
                self.env(),
                Event::Approval(Approval {
                    owner,
                    spender,
                    value: amount,
                }),
            );
        }
    }

    /// Result type
    pub type Result<T> = core::result::Result<T, PSP22Error>;
    /// Event type
    pub type Event = <TicketToken as ContractEventBase>::Type;

    /// Event emitted when a token transfer occurs.
    #[ink(event)]
    #[derive(Debug)]
    pub struct TransferEvent {
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

    impl TicketToken {
        /// Creates a new contract with the specified initial supply.
        ///
        /// Will revert if called from an account without a proper role
        #[ink(constructor)]
        pub fn new(name: String, symbol: String, total_supply: Balance) -> Self {
            let caller = Self::env().caller();
            let code_hash = Self::env()
                .own_code_hash()
                .expect("Called new on a contract with no code hash");

            let required_role = Role::Initializer(code_hash);
            let access_control = AccountId::from(ACCESS_CONTROL_PUBKEY);
            let access_control = AccessControlRef::from_account_id(access_control);

            if access_control.has_role(caller, required_role) {
                let metadata = metadata::Data {
                    name: Some(name.into()),
                    symbol: Some(symbol.into()),
                    decimals: 0,
                    ..Default::default()
                };

                let mut instance = TicketToken {
                    access_control,
                    metadata,
                    psp22: psp22::Data::default(),
                };

                instance
                    ._mint_to(instance.env().caller(), total_supply)
                    .expect("Should mint");

                instance
            } else {
                panic!("Caller is not allowed to initialize this contract");
            }
        }

        pub fn emit_event<EE: EmitEvent<Self>>(emitter: EE, event: Event) {
            emitter.emit_event(event);
        }

        /// Terminates the contract.
        ///
        /// can only be called by the contract's Admin
        #[ink(message, selector = 7)]
        pub fn terminate(&mut self) -> Result<()> {
            let caller = self.env().caller();
            let this = self.env().account_id();
            let required_role = Role::Admin(this);

            self.check_role(caller, required_role)?;
            self.env().terminate_contract(caller)
        }

        /// Returns the contract's access control contract address
        #[ink(message, selector = 8)]
        pub fn access_control(&self) -> AccountId {
            self.access_control.to_account_id()
        }

        fn check_role(&self, account: AccountId, role: Role) -> Result<()> {
            if self.access_control.has_role(account, role) {
                Ok(())
            } else {
                Err(PSP22Error::Custom(
                    format!("Role missing: {:?}", role).into(),
                ))
            }
        }

        /// Sets new access control contract address
        ///
        /// Can only be called by the contract's Admin
        #[ink(message, selector = 9)]
        pub fn set_access_control(&mut self, access_control: AccountId) -> Result<()> {
            let caller = self.env().caller();
            let this = self.env().account_id();
            let required_role = Role::Admin(this);

            self.check_role(caller, required_role)?;
            self.access_control = AccessControlRef::from_account_id(access_control);

            Ok(())
        }

        /// Returns own code hash
        #[ink(message, selector = 10)]
        pub fn code_hash(&self) -> Result<Hash> {
            Self::env().own_code_hash().map_err(|why| {
                PSP22Error::Custom(format!("Can't retrieve own code hash: {:?}", why).into())
            })
        }

        /// Upgrades contract code
        #[ink(message, selector = 11)]
        pub fn set_code(&mut self, code_hash: [u8; 32], callback: Option<Selector>) -> Result<()> {
            self.check_role(self.env().caller(), Role::Admin(self.env().account_id()))?;
            set_code_hash(&code_hash)
                .map_err(|why| PSP22Error::Custom(format!("{:?}", why).into()))?;

            // Optionally call a callback function in the new contract that performs the storage data migration.
            // By convention this function should be called `migrate`, it should take no arguments
            // and be call-able only by `this` contract's instance address.
            // To ensure the latter the `migrate` in the updated contract can e.g. check if it has an Admin role on self.
            //
            // `delegatecall` ensures that the target contract is called within the caller contracts context.
            if let Some(selector) = callback {
                build_call::<DefaultEnvironment>()
                    .delegate(Hash::from(code_hash))
                    .exec_input(ExecutionInput::new(ink::env::call::Selector::new(selector)))
                    .returns::<Result<()>>()
                    .invoke()?;
            }

            Ok(())
        }
    }
}
