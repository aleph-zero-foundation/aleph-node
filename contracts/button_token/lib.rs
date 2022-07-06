#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

pub use crate::button_token::{
    Event, ALLOWANCE_SELECTOR, BALANCE_OF_SELECTOR, TOTAL_SUPPLY_SELECTOR, TRANSFER_SELECTOR,
};

#[ink::contract]
mod button_token {

    use access_control::{traits::AccessControlled, Role, ACCESS_CONTROL_PUBKEY};
    use ink_env::Error as InkEnvError;
    use ink_lang::{codegen::EmitEvent, reflect::ContractEventBase};
    use ink_prelude::{format, string::String};
    use ink_storage::{traits::SpreadAllocate, Mapping};

    pub const TOTAL_SUPPLY_SELECTOR: [u8; 4] = [0, 0, 0, 1];
    pub const BALANCE_OF_SELECTOR: [u8; 4] = [0, 0, 0, 2];
    pub const ALLOWANCE_SELECTOR: [u8; 4] = [0, 0, 0, 3];
    pub const TRANSFER_SELECTOR: [u8; 4] = [0, 0, 0, 4];

    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct ButtonToken {
        /// Total token supply.
        total_supply: Balance,
        /// Mapping from account id to the number of owned token.
        balances: Mapping<AccountId, Balance>,
        /// Mapping of the token amount which an account is allowed to withdraw
        /// from another account.
        allowances: Mapping<(AccountId, AccountId), Balance>,
        /// access control contract
        access_control: AccountId,
    }

    /// Event emitted when a token transfer occurs.
    #[ink(event)]
    #[derive(Debug)]
    pub struct Transfer {
        #[ink(topic)]
        pub from: Option<AccountId>,
        #[ink(topic)]
        pub to: AccountId,
        pub value: Balance,
    }

    /// Event emitted when an approval occurs that `spender` is allowed to withdraw
    /// up to the amount of `value` tokens from `access_control`.
    #[ink(event)]
    #[derive(Debug)]
    pub struct Approval {
        #[ink(topic)]
        access_control: AccountId,
        #[ink(topic)]
        spender: AccountId,
        value: Balance,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Returned if not enough balance to fulfill a request is available.
        InsufficientBalance,
        /// Returned if not enough allowance to fulfill a request is available.
        InsufficientAllowance,
        /// Returned if a call to another contract has failed
        ContractCall(String),
        /// Returned if a call is made from an account with missing access conrol priviledges
        MissingRole,
    }

    /// Result type
    pub type Result<T> = core::result::Result<T, Error>;
    /// Event type
    pub type Event = <ButtonToken as ContractEventBase>::Type;

    impl AccessControlled for ButtonToken {
        type ContractError = Error;
    }

    impl ButtonToken {
        /// Creates a new contract with the specified initial supply.
        ///
        /// Will revert if called from an account without a proper role
        #[ink(constructor)]
        pub fn new(initial_supply: Balance) -> Self {
            let caller = Self::env().caller();
            let code_hash = Self::env()
                .own_code_hash()
                .expect("Called new on a contract with no code hash");
            let required_role = Role::Initializer(code_hash);

            let role_check = <Self as AccessControlled>::check_role(
                AccountId::from(ACCESS_CONTROL_PUBKEY),
                caller,
                required_role,
                |why: InkEnvError| {
                    Error::ContractCall(format!("Calling access control has failed: {:?}", why))
                },
                || Error::MissingRole,
            );

            match role_check {
                Ok(_) => ink_lang::utils::initialize_contract(|contract| {
                    Self::new_init(contract, initial_supply)
                }),
                Err(why) => panic!("Could not initialize the contract {:?}", why),
            }
        }

        /// Default initializes the contract with the specified initial supply.
        fn new_init(&mut self, initial_supply: Balance) {
            let caller = Self::env().caller();

            self.balances.insert(&caller, &initial_supply);
            self.total_supply = initial_supply;
            self.access_control = AccountId::from(ACCESS_CONTROL_PUBKEY);

            let event = Event::Transfer(Transfer {
                from: None,
                to: caller,
                value: initial_supply,
            });

            Self::emit_event(Self::env(), event)
        }

        fn emit_event<EE: EmitEvent<ButtonToken>>(emitter: EE, event: Event) {
            emitter.emit_event(event);
        }

        /// Returns the total token supply.
        #[ink(message, selector = 1)]
        pub fn total_supply(&self) -> Balance {
            self.total_supply
        }

        /// Returns the account balance for the specified `access_control`.
        ///
        /// Returns `0` if the account is non-existent.
        #[ink(message, selector = 2)]
        pub fn balance_of(&self, access_control: AccountId) -> Balance {
            self.balance_of_impl(&access_control)
        }

        /// Returns the account balance for the specified `access_control`.
        ///
        /// Returns `0` if the account is non-existent.
        ///
        /// # Note
        ///
        /// Prefer to call this method over `balance_of` since this
        /// works using references which are more efficient in Wasm.
        #[inline]
        fn balance_of_impl(&self, access_control: &AccountId) -> Balance {
            self.balances.get(access_control).unwrap_or_default()
        }

        /// Returns the amount which `spender` is still allowed to withdraw from `access_control`.
        ///
        /// Returns `0` if no allowance has been set.
        #[ink(message, selector = 3)]
        pub fn allowance(&self, access_control: AccountId, spender: AccountId) -> Balance {
            self.allowance_impl(&access_control, &spender)
        }

        /// Returns the amount which `spender` is still allowed to withdraw from `access_control`.
        ///
        /// Returns `0` if no allowance has been set.
        ///
        /// # Note
        ///
        /// Prefer to call this method over `allowance` since this
        /// works using references which are more efficient in Wasm.
        #[inline]
        fn allowance_impl(&self, access_control: &AccountId, spender: &AccountId) -> Balance {
            self.allowances
                .get((access_control, spender))
                .unwrap_or_default()
        }

        /// Transfers `value` amount of tokens from the caller's account to account `to`.
        ///
        /// On success a `Transfer` event is emitted.
        ///
        /// # Errors
        ///
        /// Returns `InsufficientBalance` error if there are not enough tokens on
        /// the caller's account balance.
        #[ink(message, selector = 4)]
        pub fn transfer(&mut self, to: AccountId, value: Balance) -> Result<()> {
            let from = self.env().caller();
            self.transfer_from_to(&from, &to, value)
        }

        /// Allows `spender` to withdraw from the caller's account multiple times, up to
        /// the `value` amount.
        ///
        /// If this function is called again it overwrites the current allowance with `value`.
        ///
        /// An `Approval` event is emitted.
        #[ink(message, selector = 5)]
        pub fn approve(&mut self, spender: AccountId, value: Balance) -> Result<()> {
            let access_control = self.env().caller();
            self.allowances.insert((&access_control, &spender), &value);

            let event = Event::Approval(Approval {
                access_control,
                spender,
                value,
            });
            Self::emit_event(self.env(), event);

            Ok(())
        }

        /// Transfers `value` tokens on the behalf of `from` to the account `to`.
        ///
        /// This can be used to allow a contract to transfer tokens on ones behalf and/or
        /// to charge fees in sub-currencies, for example.
        ///
        /// On success a `Transfer` event is emitted.
        ///
        /// # Errors
        ///
        /// Returns `InsufficientAllowance` error if there are not enough tokens allowed
        /// for the caller to withdraw from `from`.
        ///
        /// Returns `InsufficientBalance` error if there are not enough tokens on
        /// the account balance of `from`.
        #[ink(message, selector = 6)]
        pub fn transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            value: Balance,
        ) -> Result<()> {
            let caller = self.env().caller();
            let allowance = self.allowance_impl(&from, &caller);
            if allowance < value {
                return Err(Error::InsufficientAllowance);
            }
            self.transfer_from_to(&from, &to, value)?;
            self.allowances
                .insert((&from, &caller), &(allowance - value));
            Ok(())
        }

        /// Transfers `value` amount of tokens from the caller's account to account `to`.
        ///
        /// On success a `Transfer` event is emitted.
        ///
        /// # Errors
        ///
        /// Returns `InsufficientBalance` error if there are not enough tokens on
        /// the caller's account balance.
        fn transfer_from_to(
            &mut self,
            from: &AccountId,
            to: &AccountId,
            value: Balance,
        ) -> Result<()> {
            let from_balance = self.balance_of_impl(from);
            if from_balance < value {
                return Err(Error::InsufficientBalance);
            }

            self.balances.insert(from, &(from_balance - value));
            let to_balance = self.balance_of_impl(to);
            self.balances.insert(to, &(to_balance + value));

            let event = Event::Transfer(Transfer {
                from: Some(*from),
                to: *to,
                value,
            });
            Self::emit_event(self.env(), event);

            Ok(())
        }

        /// Terminates the contract.
        ///
        /// can only be called by the contract's Owner
        #[ink(message, selector = 7)]
        pub fn terminate(&mut self) -> Result<()> {
            let caller = self.env().caller();
            let this = self.env().account_id();
            let required_role = Role::Owner(this);

            self.check_role(caller, required_role)?;

            self.env().terminate_contract(caller)
        }

        /// Returns the contract's access control contract address
        #[ink(message, selector = 8)]
        pub fn access_control(&self) -> AccountId {
            self.access_control
        }

        fn check_role(&self, account: AccountId, role: Role) -> Result<()> {
            <Self as AccessControlled>::check_role(
                self.access_control,
                account,
                role,
                |why: InkEnvError| {
                    Error::ContractCall(format!("Calling access control has failed: {:?}", why))
                },
                || Error::MissingRole,
            )
        }

        /// Sets new access control contract address
        ///
        /// Can only be called by the contract's Owner
        #[ink(message, selector = 9)]
        pub fn set_access_control(&mut self, access_control: AccountId) -> Result<()> {
            let caller = self.env().caller();
            let this = self.env().account_id();
            let required_role = Role::Owner(this);

            self.check_role(caller, required_role)?;

            self.access_control = access_control;
            Ok(())
        }

        /// Returns own code hash
        #[ink(message, selector = 10)]
        pub fn code_hash(&self) -> Result<Hash> {
            Self::env().own_code_hash().map_err(|why| {
                Error::ContractCall(format!("Calling control has failed: {:?}", why))
            })
        }
    }

    /*
    #[cfg(test)]
    mod tests {
        use super::*;

        use ink_env::Clear;
        use ink_lang as ink;

        fn assert_transfer_event(
            event: &ink_env::test::EmittedEvent,
            expected_from: Option<AccountId>,
            expected_to: AccountId,
            expected_value: Balance,
        ) {
            let decoded_event = <Event as scale::Decode>::decode(&mut &event.data[..])
                .expect("encountered invalid contract event data buffer");
            if let Event::Transfer(Transfer { from, to, value }) = decoded_event {
                assert_eq!(from, expected_from, "encountered invalid Transfer.from");
                assert_eq!(to, expected_to, "encountered invalid Transfer.to");
                assert_eq!(value, expected_value, "encountered invalid Trasfer.value");
            } else {
                panic!("encountered unexpected event kind: expected a Transfer event")
            }
            let expected_topics = vec![
                encoded_into_hash(&PrefixedValue {
                    value: b"ButtonToken::Transfer",
                    prefix: b"",
                }),
                encoded_into_hash(&PrefixedValue {
                    prefix: b"ButtonToken::Transfer::from",
                    value: &expected_from,
                }),
                encoded_into_hash(&PrefixedValue {
                    prefix: b"ButtonToken::Transfer::to",
                    value: &expected_to,
                }),
            ];

            let topics = event.topics.clone();
            for (n, (actual_topic, expected_topic)) in
                topics.iter().zip(expected_topics).enumerate()
            {
                let mut topic_hash = Hash::clear();
                let len = actual_topic.len();
                topic_hash.as_mut()[0..len].copy_from_slice(&actual_topic[0..len]);

                assert_eq!(
                    topic_hash, expected_topic,
                    "encountered invalid topic at {}",
                    n
                );
            }
        }

        /// The default constructor does its job.
        #[ink::test]
        fn new_works() {
            // Constructor works.
            let _erc20 = ButtonToken::new(100);

            // Transfer event triggered during initial construction.
            let emitted_events = ink_env::test::recorded_events().collect::<Vec<_>>();
            assert_eq!(1, emitted_events.len());

            assert_transfer_event(&emitted_events[0], None, AccountId::from([0x01; 32]), 100);
        }

        /// The total supply was applied.
        #[ink::test]
        fn total_supply_works() {
            let erc20 = ButtonToken::new(100);
            // Get the token total supply.
            assert_eq!(erc20.total_supply(), 100);
        }

        /// Get the actual balance of an account.
        #[ink::test]
        fn balance_of_works() {
            // Constructor works
            let erc20 = ButtonToken::new(100);
            // Transfer event triggered during initial construction
            let emitted_events = ink_env::test::recorded_events().collect::<Vec<_>>();
            assert_transfer_event(&emitted_events[0], None, AccountId::from([0x01; 32]), 100);
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            // Alice owns all the tokens on contract instantiation
            assert_eq!(erc20.balance_of(accounts.alice), 100);
            // Bob does not owns tokens
            assert_eq!(erc20.balance_of(accounts.bob), 0);
        }

        #[ink::test]
        fn transfer_works() {
            // Constructor works.
            let mut erc20 = ButtonToken::new(100);
            // Transfer event triggered during initial construction.
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();

            assert_eq!(erc20.balance_of(accounts.bob), 0);
            // Alice transfers 10 tokens to Bob.
            assert_eq!(erc20.transfer(accounts.bob, 10), Ok(()));
            // Bob owns 10 tokens.
            assert_eq!(erc20.balance_of(accounts.bob), 10);

            let emitted_events = ink_env::test::recorded_events().collect::<Vec<_>>();
            assert_eq!(emitted_events.len(), 2);
            // Check first transfer event related to ERC-20 instantiation.
            assert_transfer_event(&emitted_events[0], None, AccountId::from([0x01; 32]), 100);
            // Check the second transfer event relating to the actual trasfer.
            assert_transfer_event(
                &emitted_events[1],
                Some(AccountId::from([0x01; 32])),
                AccountId::from([0x02; 32]),
                10,
            );
        }

        #[ink::test]
        fn invalid_transfer_should_fail() {
            // Constructor works.
            let mut erc20 = ButtonToken::new(100);
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();

            assert_eq!(erc20.balance_of(accounts.bob), 0);

            // Set the contract as callee and Bob as caller.
            let contract = ink_env::account_id::<ink_env::DefaultEnvironment>();
            ink_env::test::set_callee::<ink_env::DefaultEnvironment>(contract);
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(accounts.bob);

            // Bob fails to transfers 10 tokens to Eve.
            assert_eq!(
                erc20.transfer(accounts.eve, 10),
                Err(Error::InsufficientBalance)
            );
            // Alice owns all the tokens.
            assert_eq!(erc20.balance_of(accounts.alice), 100);
            assert_eq!(erc20.balance_of(accounts.bob), 0);
            assert_eq!(erc20.balance_of(accounts.eve), 0);

            // Transfer event triggered during initial construction.
            let emitted_events = ink_env::test::recorded_events().collect::<Vec<_>>();
            assert_eq!(emitted_events.len(), 1);
            assert_transfer_event(&emitted_events[0], None, AccountId::from([0x01; 32]), 100);
        }

        #[ink::test]
        fn transfer_from_works() {
            // Constructor works.
            let mut erc20 = ButtonToken::new(100);
            // Transfer event triggered during initial construction.
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();

            // Bob fails to transfer tokens owned by Alice.
            assert_eq!(
                erc20.transfer_from(accounts.alice, accounts.eve, 10),
                Err(Error::InsufficientAllowance)
            );
            // Alice approves Bob for token transfers on her behalf.
            assert_eq!(erc20.approve(accounts.bob, 10), Ok(()));

            // The approve event takes place.
            assert_eq!(ink_env::test::recorded_events().count(), 2);

            // Set the contract as callee and Bob as caller.
            let contract = ink_env::account_id::<ink_env::DefaultEnvironment>();
            ink_env::test::set_callee::<ink_env::DefaultEnvironment>(contract);
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(accounts.bob);

            // Bob transfers tokens from Alice to Eve.
            assert_eq!(
                erc20.transfer_from(accounts.alice, accounts.eve, 10),
                Ok(())
            );
            // Eve owns tokens.
            assert_eq!(erc20.balance_of(accounts.eve), 10);

            // Check all transfer events that happened during the previous calls:
            let emitted_events = ink_env::test::recorded_events().collect::<Vec<_>>();
            assert_eq!(emitted_events.len(), 3);
            assert_transfer_event(&emitted_events[0], None, AccountId::from([0x01; 32]), 100);
            // The second event `emitted_events[1]` is an Approve event that we skip checking.
            assert_transfer_event(
                &emitted_events[2],
                Some(AccountId::from([0x01; 32])),
                AccountId::from([0x05; 32]),
                10,
            );
        }

        #[ink::test]
        fn allowance_must_not_change_on_failed_transfer() {
            let mut erc20 = ButtonToken::new(100);
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();

            // Alice approves Bob for token transfers on her behalf.
            let alice_balance = erc20.balance_of(accounts.alice);
            let initial_allowance = alice_balance + 2;
            assert_eq!(erc20.approve(accounts.bob, initial_allowance), Ok(()));

            // Get contract address.
            let callee = ink_env::account_id::<ink_env::DefaultEnvironment>();
            ink_env::test::set_callee::<ink_env::DefaultEnvironment>(callee);
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(accounts.bob);

            // Bob tries to transfer tokens from Alice to Eve.
            let emitted_events_before = ink_env::test::recorded_events().count();
            assert_eq!(
                erc20.transfer_from(accounts.alice, accounts.eve, alice_balance + 1),
                Err(Error::InsufficientBalance)
            );
            // Allowance must have stayed the same
            assert_eq!(
                erc20.allowance(accounts.alice, accounts.bob),
                initial_allowance
            );
            // No more events must have been emitted
            assert_eq!(
                emitted_events_before,
                ink_env::test::recorded_events().count()
            )
        }

        /// For calculating the event topic hash.
        struct PrefixedValue<'a, 'b, T> {
            pub prefix: &'a [u8],
            pub value: &'b T,
        }

        impl<X> scale::Encode for PrefixedValue<'_, '_, X>
        where
            X: scale::Encode,
        {
            #[inline]
            fn size_hint(&self) -> usize {
                self.prefix.size_hint() + self.value.size_hint()
            }

            #[inline]
            fn encode_to<T: scale::Output + ?Sized>(&self, dest: &mut T) {
                self.prefix.encode_to(dest);
                self.value.encode_to(dest);
            }
        }

        fn encoded_into_hash<T>(entity: &T) -> Hash
        where
            T: scale::Encode,
        {
            use ink_env::{
                hash::{Blake2x256, CryptoHash, HashOutput},
                Clear,
            };
            let mut result = Hash::clear();
            let len_result = result.as_ref().len();
            let encoded = entity.encode();
            let len_encoded = encoded.len();
            if len_encoded <= len_result {
                result.as_mut()[..len_encoded].copy_from_slice(&encoded);
                return result;
            }
            let mut hash_output = <<Blake2x256 as HashOutput>::Type as Default>::default();
            <Blake2x256 as CryptoHash>::hash(&encoded, &mut hash_output);
            let copy_len = core::cmp::min(hash_output.len(), len_result);
            result.as_mut()[0..copy_len].copy_from_slice(&hash_output[0..copy_len]);
            result
        }
    }*/
}
