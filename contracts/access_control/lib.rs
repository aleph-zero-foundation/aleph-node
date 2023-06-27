#![cfg_attr(not(feature = "std"), no_std, no_main)]
#![allow(clippy::let_unit_value)]

pub use crate::access_control::{
    AccessControl, AccessControlError, AccessControlRef, ACCESS_CONTROL_PUBKEY,
    CHECK_ROLE_SELECTOR, HAS_ROLE_SELECTOR,
};
pub mod roles;

use ink::env::{DefaultEnvironment, Environment};

type AccountId = <DefaultEnvironment as Environment>::AccountId;
type Hash = <DefaultEnvironment as Environment>::Hash;

#[ink::contract]
mod access_control {
    use ink::{
        codegen::EmitEvent,
        env::{
            call::{build_call, ExecutionInput},
            set_code_hash, Error as InkEnvError,
        },
        prelude::{format, string::String},
        reflect::ContractEventBase,
        storage::{traits::ManualKey, Mapping},
    };
    use scale::{Decode, Encode};
    use shared_traits::Selector;

    use crate::{roles::Role, DefaultEnvironment};
    // address placeholder, to be set in the bytecode
    // 4465614444656144446561444465614444656144446561444465614444656144 => 5DcPEG9AQ4Y9Lo9C5WXuKJDDawens77jWxZ6zGChnm8y8FUX
    pub const ACCESS_CONTROL_PUBKEY: [u8; 32] = *b"DeaDDeaDDeaDDeaDDeaDDeaDDeaDDeaD";
    pub const HAS_ROLE_SELECTOR: [u8; 4] = [0, 0, 0, 3];
    pub const CHECK_ROLE_SELECTOR: [u8; 4] = [0, 0, 0, 5];

    #[ink(storage)]
    pub struct AccessControl {
        pub privileges: Mapping<(AccountId, Role), (), ManualKey<0x50524956>>,
    }

    #[ink(event)]
    #[derive(Debug)]
    pub struct RoleGranted {
        #[ink(topic)]
        by: AccountId,
        #[ink(topic)]
        to: AccountId,
        #[ink(topic)]
        role: Role,
    }

    #[ink(event)]
    #[derive(Debug)]
    pub struct RoleRevoked {
        #[ink(topic)]
        by: AccountId,
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        role: Role,
    }

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum AccessControlError {
        InkEnvError(String),
        MissingRole(Role),
    }

    impl From<InkEnvError> for AccessControlError {
        fn from(why: InkEnvError) -> Self {
            Self::InkEnvError(format!("{:?}", why))
        }
    }

    /// Result type
    pub type Result<T> = core::result::Result<T, AccessControlError>;
    /// Event type
    pub type Event = <AccessControl as ContractEventBase>::Type;

    impl AccessControl {
        /// Creates a new contract.
        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();
            let this = Self::env().account_id();

            let mut privileges = Mapping::default();
            privileges.insert((caller, Role::Admin(this)), &());

            Self { privileges }
        }

        /// Gives a role to an account
        ///
        /// Can only be called by an account with an admin role on this contract
        #[ink(message, selector = 1)]
        pub fn grant_role(&mut self, account: AccountId, role: Role) -> Result<()> {
            let key = (account, role);
            if !self.privileges.contains(key) {
                let caller = self.env().caller();
                let this = self.env().account_id();
                self.check_role(caller, Role::Admin(this))?;
                self.privileges.insert(key, &());

                Self::emit_event(
                    self.env(),
                    Event::RoleGranted(RoleGranted {
                        by: caller,
                        to: account,
                        role,
                    }),
                );
            }

            Ok(())
        }

        /// Revokes a role from an account
        ///
        /// Can only be called by an admin role on this contract
        #[ink(message, selector = 2)]
        pub fn revoke_role(&mut self, account: AccountId, role: Role) -> Result<()> {
            let caller = self.env().caller();
            let this = self.env().account_id();
            self.check_role(caller, Role::Admin(this))?;
            self.privileges.remove((account, role));

            Self::emit_event(
                self.env(),
                Event::RoleRevoked(RoleRevoked {
                    by: caller,
                    from: account,
                    role,
                }),
            );

            Ok(())
        }

        /// Returns true if account has a role
        #[ink(message, selector = 3)]
        pub fn has_role(&self, account: AccountId, role: Role) -> bool {
            self.privileges.contains((account, role))
        }

        /// Terminates the contract.
        ///
        /// can only be called by the contract owner
        #[ink(message, selector = 4)]
        pub fn terminate(&mut self) -> Result<()> {
            let caller = self.env().caller();
            self.check_role(caller, Role::Admin(self.env().account_id()))?;
            self.env().terminate_contract(caller)
        }

        /// Result wrapped version of `has_role`
        ///
        /// Returns Error variant MissingRole(Role) if the account does not carry a role
        #[ink(message, selector = 5)]
        pub fn check_role(&self, account: AccountId, role: Role) -> Result<()> {
            if !self.has_role(account, role) {
                return Err(AccessControlError::MissingRole(role));
            }
            Ok(())
        }

        /// Upgrades contract code
        #[ink(message, selector = 6)]
        pub fn set_code(&mut self, code_hash: [u8; 32], callback: Option<Selector>) -> Result<()> {
            self.check_role(self.env().caller(), Role::Admin(self.env().account_id()))?;
            set_code_hash(&code_hash)?;

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

        fn emit_event<EE: EmitEvent<Self>>(emitter: EE, event: Event) {
            emitter.emit_event(event);
        }
    }

    impl Default for AccessControl {
        fn default() -> Self {
            Self::new()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn access_control() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            let alice = accounts.alice;
            let bob = accounts.bob;
            let charlie = accounts.charlie;
            let contract_address = accounts.django;

            // alice deploys the access control contract
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(alice);
            ink::env::test::set_callee::<ink::env::DefaultEnvironment>(contract_address);
            ink::env::test::set_account_balance::<ink::env::DefaultEnvironment>(
                contract_address,
                100,
            );
            let mut access_control = AccessControl::new();

            // alice should be admin
            assert!(
                access_control.has_role(alice, Role::Admin(contract_address)),
                "deployer is not admin"
            );

            // alice grants admin rights to bob
            assert!(
                access_control
                    .grant_role(bob, Role::Admin(contract_address))
                    .is_ok(),
                "Alice's grant_role call failed"
            );

            assert!(
                access_control.has_role(bob, Role::Admin(contract_address)),
                "Bob is not admin"
            );

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(charlie);
            ink::env::test::set_callee::<ink::env::DefaultEnvironment>(contract_address);

            // charlie tries granting admin rights to himself
            assert!(
                access_control
                    .grant_role(charlie, Role::Admin(contract_address))
                    .is_err(),
                "grant_role should fail"
            );

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(alice);
            ink::env::test::set_callee::<ink::env::DefaultEnvironment>(contract_address);
            // alice gives a custom role to bob
            assert!(
                access_control
                    .grant_role(
                        bob,
                        Role::Custom(contract_address, [0x43, 0x53, 0x54, 0x4D])
                    )
                    .is_ok(),
                "custom grant_role should work"
            );

            assert!(
                access_control.has_role(
                    bob,
                    Role::Custom(contract_address, [0x43, 0x53, 0x54, 0x4D])
                ),
                "bob should have a custom role"
            );

            // test terminating
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(alice);
            ink::env::test::set_callee::<ink::env::DefaultEnvironment>(contract_address);

            let should_terminate = move || {
                access_control
                    .terminate()
                    .expect("Calling terminate failed")
            };

            ink::env::test::assert_contract_termination::<ink::env::DefaultEnvironment, _>(
                should_terminate,
                alice,
                100,
            );
        }
    }
}
