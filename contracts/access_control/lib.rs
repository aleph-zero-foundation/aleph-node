#![cfg_attr(not(feature = "std"), no_std)]

pub use crate::access_control::{
    AccessControlError, Role, ACCESS_CONTROL_PUBKEY, CHECK_ROLE_SELECTOR, HAS_ROLE_SELECTOR,
};
pub mod traits;
use ink_lang as ink;

#[ink::contract]
mod access_control {

    use ink_lang::{codegen::EmitEvent, reflect::ContractEventBase};
    use ink_storage::{
        traits::{PackedLayout, SpreadAllocate, SpreadLayout},
        Mapping,
    };
    use scale::{Decode, Encode};

    // address placeholder, set in the bytecode
    // 4465614444656144446561444465614444656144446561444465614444656144 => 5DcPEG9AQ4Y9Lo9C5WXuKJDDawens77jWxZ6zGChnm8y8FUX
    pub const ACCESS_CONTROL_PUBKEY: [u8; 32] = *b"DeaDDeaDDeaDDeaDDeaDDeaDDeaDDeaD";
    pub const HAS_ROLE_SELECTOR: [u8; 4] = [0, 0, 0, 3];
    pub const CHECK_ROLE_SELECTOR: [u8; 4] = [0, 0, 0, 5];

    #[derive(Debug, Encode, Decode, Clone, Copy, SpreadLayout, PackedLayout, PartialEq, Eq)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout)
    )]
    pub enum Role {
        /// Indicates a superuser.
        Admin(AccountId),
        /// Indicates account can terminate a contract.
        Owner(AccountId),
        /// Indicates account can initialize a contract from a given code hash.
        Initializer(Hash),
    }

    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct AccessControl {
        /// Stores a de-facto hashset of user accounts and their roles
        pub priviledges: Mapping<(AccountId, Role), ()>,
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
        MissingRole(Role),
    }

    /// Result type    
    pub type Result<T> = core::result::Result<T, AccessControlError>;
    /// Event type
    pub type Event = <AccessControl as ContractEventBase>::Type;

    impl AccessControl {
        /// Creates a new contract.
        #[ink(constructor)]
        pub fn new() -> Self {
            // This call is required in order to correctly initialize the
            // `Mapping`s of our contract.
            ink_lang::utils::initialize_contract(|contract| Self::new_init(contract))
        }

        /// Initializes the contract.
        ///
        /// caller is granted admin and owner piviledges
        fn new_init(&mut self) {
            let caller = Self::env().caller();
            let this = Self::env().account_id();
            self.priviledges.insert((caller, Role::Admin(this)), &());
            self.priviledges.insert((caller, Role::Owner(this)), &());
        }

        /// Gives a role to an account
        ///
        /// Can only be called by an account with an admin role on this contract
        #[ink(message, selector = 1)]
        pub fn grant_role(&mut self, account: AccountId, role: Role) -> Result<()> {
            let key = (account, role);
            if !self.priviledges.contains(key) {
                let caller = self.env().caller();
                let this = self.env().account_id();
                self.check_role(caller, Role::Admin(this))?;
                self.priviledges.insert(key, &());

                let event = Event::RoleGranted(RoleGranted {
                    by: caller,
                    to: account,
                    role,
                });
                Self::emit_event(self.env(), event);
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
            self.priviledges.remove((account, role));

            let event = Event::RoleRevoked(RoleRevoked {
                by: caller,
                from: account,
                role,
            });
            Self::emit_event(self.env(), event);

            Ok(())
        }

        /// Returns true if account has a role
        #[ink(message, selector = 3)]
        pub fn has_role(&self, account: AccountId, role: Role) -> bool {
            self.priviledges.contains((account, role))
        }

        /// Terminates the contract.
        ///
        /// can only be called by the contract owner
        #[ink(message, selector = 4)]
        pub fn terminate(&mut self) -> Result<()> {
            let caller = self.env().caller();
            let this = self.env().account_id();
            self.check_role(caller, Role::Owner(this))?;
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

        fn emit_event<EE: EmitEvent<Self>>(emitter: EE, event: Event) {
            emitter.emit_event(event);
        }
    }

    #[cfg(test)]
    mod tests {
        use ink_lang as ink;

        use super::*;

        #[ink::test]
        fn access_control() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();

            let alice = accounts.alice;
            let bob = accounts.bob;
            let charlie = accounts.charlie;
            let contract_address = accounts.django;

            // alice deploys the access control contract
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(alice);
            ink_env::test::set_callee::<ink_env::DefaultEnvironment>(contract_address);
            ink_env::test::set_account_balance::<ink_env::DefaultEnvironment>(
                contract_address,
                100,
            );
            let mut access_control = AccessControl::new();

            // alice should be admin
            assert!(
                access_control.has_role(alice, Role::Admin(contract_address)),
                "deployer is not admin"
            );

            // alice should be owner
            assert!(
                access_control.has_role(alice, Role::Owner(contract_address)),
                "deployer is not owner"
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

            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(charlie);
            ink_env::test::set_callee::<ink_env::DefaultEnvironment>(contract_address);

            // charlie tries granting admin rights to himself
            assert!(
                access_control
                    .grant_role(charlie, Role::Admin(contract_address))
                    .is_err(),
                "grant_role should fail"
            );

            // test terminating
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(alice);
            ink_env::test::set_callee::<ink_env::DefaultEnvironment>(contract_address);

            let should_terminate = move || {
                access_control
                    .terminate()
                    .expect("Calling terminate failed")
            };

            ink_env::test::assert_contract_termination::<ink_env::DefaultEnvironment, _>(
                should_terminate,
                alice,
                100,
            );
        }
    }
}
