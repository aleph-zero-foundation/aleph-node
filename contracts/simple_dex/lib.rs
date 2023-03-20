#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::let_unit_value)]

/// Simple DEX contract
///
/// This contract is based on Balancer multi asset LP design and all formulas are taken from the Balancer's whitepaper (https://balancer.fi/whitepaper.pdf)
/// It has one pool with PSP22 tokens with equal weights
///
/// Swaps can be performed between all pairs in the pool whitelisted for trading
/// Liquidity provisioning is limited to designated accounts only and works as deposits / withdrawals of arbitrary composition.

#[ink::contract]
mod simple_dex {
    use access_control::{roles::Role, AccessControlRef, ACCESS_CONTROL_PUBKEY};
    use ink::{
        codegen::EmitEvent,
        env::{call::FromAccountId, CallFlags, Error as InkEnvError},
        prelude::{format, string::String, vec, vec::Vec},
        reflect::ContractEventBase,
        LangError, ToAccountId,
    };
    use openbrush::{
        contracts::{psp22::PSP22Ref, traits::errors::PSP22Error},
        storage::Mapping,
        traits::Storage,
    };

    type Event = <SimpleDex as ContractEventBase>::Type;

    pub const LIQUIDITY_PROVIDER: [u8; 4] = [0x4C, 0x51, 0x54, 0x59];

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct SwapPair {
        pub from: AccountId,
        pub to: AccountId,
    }

    impl SwapPair {
        pub fn new(from: AccountId, to: AccountId) -> Self {
            Self { from, to }
        }
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum DexError {
        PSP22(PSP22Error),
        InsufficientAllowanceOf(AccountId),
        Arithmethic,
        WrongParameterValue,
        MissingRole(AccountId, Role),
        InkEnv(String),
        CrossContractCall(String),
        TooMuchSlippage,
        NotEnoughLiquidityOf(AccountId),
        UnsupportedSwapPair(SwapPair),
    }

    impl From<PSP22Error> for DexError {
        fn from(e: PSP22Error) -> Self {
            DexError::PSP22(e)
        }
    }

    impl From<InkEnvError> for DexError {
        fn from(why: InkEnvError) -> Self {
            DexError::InkEnv(format!("{:?}", why))
        }
    }

    impl From<LangError> for DexError {
        fn from(why: LangError) -> Self {
            DexError::CrossContractCall(format!("{:?}", why))
        }
    }

    #[ink(event)]
    pub struct Deposited {
        caller: AccountId,
        #[ink(topic)]
        token: AccountId,
        amount: Balance,
    }

    #[ink(event)]
    pub struct SwapPairAdded {
        #[ink(topic)]
        pair: SwapPair,
    }

    #[ink(event)]
    pub struct SwapPairRemoved {
        #[ink(topic)]
        pair: SwapPair,
    }

    #[ink(event)]
    pub struct Swapped {
        caller: AccountId,
        #[ink(topic)]
        token_in: AccountId,
        #[ink(topic)]
        token_out: AccountId,
        amount_in: Balance,
        amount_out: Balance,
    }

    #[ink(event)]
    pub struct SwapFeeSet {
        #[ink(topic)]
        caller: AccountId,
        swap_fee_percentage: u128,
    }

    #[ink(storage)]
    #[derive(Storage)]
    pub struct SimpleDex {
        pub swap_fee_percentage: u128,
        pub access_control: AccessControlRef,
        // a set of pairs that are availiable for swapping between
        pub swap_pairs: Mapping<SwapPair, ()>,
    }

    impl SimpleDex {
        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();
            let code_hash = Self::env()
                .own_code_hash()
                .expect("Called new on a contract with no code hash");
            let required_role = Role::Initializer(code_hash);
            let access_control = AccountId::from(ACCESS_CONTROL_PUBKEY);
            let access_control = AccessControlRef::from_account_id(access_control);

            if access_control.has_role(caller, required_role) {
                Self {
                    swap_fee_percentage: 0,
                    access_control,
                    swap_pairs: Mapping::default(),
                }
            } else {
                panic!("Caller is not allowed to initialize this contract");
            }
        }

        /// Swaps the a specified amount of one of the pool's PSP22 tokens to another PSP22 token
        /// Calling account needs to give allowance to the DEX contract to spend amount_token_in of token_in on its behalf
        /// before executing this tx.
        #[ink(message)]
        pub fn swap(
            &mut self,
            token_in: AccountId,
            token_out: AccountId,
            amount_token_in: Balance,
            min_amount_token_out: Balance,
        ) -> Result<(), DexError> {
            let this = self.env().account_id();
            let caller = self.env().caller();

            let balance_token_out = self.balance_of(token_out, this);
            if balance_token_out < min_amount_token_out {
                // throw early if we cannot support this swap anyway due to liquidity being too low
                return Err(DexError::NotEnoughLiquidityOf(token_out));
            }

            let swap_pair = SwapPair::new(token_in, token_out);
            if !self.swap_pairs.contains(&swap_pair) {
                return Err(DexError::UnsupportedSwapPair(swap_pair));
            }

            // check allowance
            if self.allowance(token_in, caller, this) < amount_token_in {
                return Err(DexError::InsufficientAllowanceOf(token_in));
            }

            let amount_token_out = self.out_given_in(token_in, token_out, amount_token_in)?;

            if amount_token_out < min_amount_token_out {
                // thrown if too much slippage occured before this tx gets executed
                // as a sandwich attack prevention
                return Err(DexError::TooMuchSlippage);
            }

            // transfer token_in from user to the contract
            self.transfer_from_tx(token_in, caller, this, amount_token_in)?;
            // transfer token_out from contract to user
            self.transfer_tx(token_out, caller, amount_token_out)?;

            // emit event
            Self::emit_event(
                self.env(),
                Event::Swapped(Swapped {
                    caller,
                    token_in,
                    token_out,
                    amount_in: amount_token_in,
                    amount_out: amount_token_out,
                }),
            );

            Ok(())
        }

        /// Liquidity deposit
        ///
        /// Can only be performed by an account with a LiquidityProvider role
        /// Caller needs to give at least the passed amount of allowance to the contract to spend the deposited tokens on his behalf
        /// prior to executing this tx
        #[ink(message)]
        pub fn deposit(&mut self, deposits: Vec<(AccountId, Balance)>) -> Result<(), DexError> {
            let this = self.env().account_id();
            let caller = self.env().caller();

            // check role, only designated account can add liquidity
            self.check_role(caller, Role::Custom(this, LIQUIDITY_PROVIDER))?;

            deposits
                .into_iter()
                .try_for_each(|(token_in, amount)| -> Result<(), DexError> {
                    // transfer token_in from the caller to the contract
                    // will revert if the contract does not have enough allowance from the caller
                    // in which case the whole tx is reverted
                    self.transfer_from_tx(token_in, caller, this, amount)?;

                    Self::emit_event(
                        self.env(),
                        Event::Deposited(Deposited {
                            caller,
                            token: token_in,
                            amount,
                        }),
                    );

                    Ok(())
                })?;

            Ok(())
        }

        #[ink(message)]
        pub fn withdrawal(
            &mut self,
            withdrawals: Vec<(AccountId, Balance)>,
        ) -> Result<(), DexError> {
            let this = self.env().account_id();
            let caller = self.env().caller();

            // check role, only designated account can remove liquidity
            self.check_role(caller, Role::Custom(this, LIQUIDITY_PROVIDER))?;

            withdrawals.into_iter().try_for_each(
                |(token_out, amount)| -> Result<(), DexError> {
                    // transfer token_out from the contract to the caller
                    self.transfer_tx(token_out, caller, amount)?;
                    Ok(())
                },
            )?;

            Ok(())
        }

        /// Alters the swap_fee parameter
        ///
        /// Can only be called by the contract's Admin.
        #[ink(message)]
        pub fn set_swap_fee_percentage(
            &mut self,
            swap_fee_percentage: u128,
        ) -> Result<(), DexError> {
            if swap_fee_percentage.gt(&100) {
                return Err(DexError::WrongParameterValue);
            }

            let caller = self.env().caller();
            let this = self.env().account_id();

            self.check_role(caller, Role::Admin(this))?;

            // emit event
            Self::emit_event(
                self.env(),
                Event::SwapFeeSet(SwapFeeSet {
                    caller,
                    swap_fee_percentage,
                }),
            );

            self.swap_fee_percentage = swap_fee_percentage;
            Ok(())
        }

        /// Returns current value of the swap_fee_percentage parameter
        #[ink(message)]
        pub fn swap_fee_percentage(&self) -> Balance {
            self.swap_fee_percentage
        }

        /// Sets access_control to a new contract address
        ///
        /// Potentially very destructive, can only be called by the contract's Admin.
        #[ink(message)]
        pub fn set_access_control(&mut self, access_control: AccountId) -> Result<(), DexError>
        where
            Self: AccessControlled,
        {
            let caller = self.env().caller();
            let this = self.env().account_id();

            self.check_role(caller, Role::Admin(this))?;

            self.access_control = AccessControlRef::from_account_id(access_control);
            Ok(())
        }

        /// Returns current address of the AccessControl contract that holds the account priviledges for this DEX
        #[ink(message)]
        pub fn access_control(&self) -> AccountId {
            self.access_control.to_account_id()
        }

        /// Whitelists a token pair for swapping between
        ///
        /// Token pair is understood as a swap between tokens in one direction
        /// Can only be called by an Admin
        #[ink(message)]
        pub fn add_swap_pair(&mut self, from: AccountId, to: AccountId) -> Result<(), DexError> {
            let caller = self.env().caller();
            let this = self.env().account_id();
            self.check_role(caller, Role::Admin(this))?;

            let pair = SwapPair::new(from, to);
            self.swap_pairs.insert(&pair, &());

            Self::emit_event(self.env(), Event::SwapPairAdded(SwapPairAdded { pair }));

            Ok(())
        }

        /// Returns true if a pair of tokens is whitelisted for swapping between
        #[ink(message)]
        pub fn can_swap_pair(&self, from: AccountId, to: AccountId) -> bool {
            self.swap_pairs.contains(&SwapPair::new(from, to))
        }

        /// Blacklists a token pair from swapping
        ///
        /// Token pair is understood as a swap between tokens in one direction
        /// Can only be called by an Admin
        #[ink(message)]
        pub fn remove_swap_pair(&mut self, from: AccountId, to: AccountId) -> Result<(), DexError> {
            let caller = self.env().caller();
            let this = self.env().account_id();
            self.check_role(caller, Role::Admin(this))?;

            let pair = SwapPair::new(from, to);
            self.swap_pairs.remove(&pair);

            Self::emit_event(self.env(), Event::SwapPairRemoved(SwapPairRemoved { pair }));

            Ok(())
        }

        /// Terminates the contract.
        ///
        /// Can only be called by the contract's Admin.
        #[ink(message)]
        pub fn terminate(&mut self) -> Result<(), DexError> {
            let caller = self.env().caller();
            let this = self.env().account_id();
            self.check_role(caller, Role::Admin(this))?;
            self.env().terminate_contract(caller)
        }

        /// Returns own code hash
        #[ink(message)]
        pub fn code_hash(&self) -> Result<Hash, DexError> {
            self.env()
                .own_code_hash()
                .map_err(|why| DexError::InkEnv(format!("Can't retrieve own code hash: {:?}", why)))
        }

        /// Swap trade output given a curve with equal token weights
        ///
        /// B_0 - (100 * B_0 * B_i) / (100 * (B_i + A_i) - A_i * swap_fee)
        /// where swap_fee (integer) is a percentage of the trade that goes towards the pool
        /// and is used to pay the liquidity providers
        #[ink(message)]
        pub fn out_given_in(
            &self,
            token_in: AccountId,
            token_out: AccountId,
            amount_token_in: Balance,
        ) -> Result<Balance, DexError> {
            let this = self.env().account_id();
            let balance_token_in = self.balance_of(token_in, this);
            let balance_token_out = self.balance_of(token_out, this);

            Self::_out_given_in(
                amount_token_in,
                balance_token_in,
                balance_token_out,
                self.swap_fee_percentage,
            )
        }

        fn _out_given_in(
            amount_token_in: Balance,
            balance_token_in: Balance,
            balance_token_out: Balance,
            swap_fee_percentage: Balance,
        ) -> Result<Balance, DexError> {
            let op0 = amount_token_in
                .checked_mul(swap_fee_percentage)
                .ok_or(DexError::Arithmethic)?;

            let op1 = balance_token_in
                .checked_add(amount_token_in)
                .and_then(|result| result.checked_mul(100))
                .ok_or(DexError::Arithmethic)?;

            let op2 = op1.checked_sub(op0).ok_or(DexError::Arithmethic)?;

            let op3 = balance_token_in
                .checked_mul(balance_token_out)
                .and_then(|result| result.checked_mul(100))
                .ok_or(DexError::Arithmethic)?;

            let op4 = op3.checked_div(op2).ok_or(DexError::Arithmethic)?;

            balance_token_out
                .checked_sub(op4)
                // If the division is not even, leave the 1 unit of dust in the exchange instead of paying it out.
                .and_then(|result| result.checked_sub((op3 % op2 > 0).into()))
                .ok_or(DexError::Arithmethic)
        }

        /// Transfers a given amount of a PSP22 token to a specified using the callers own balance
        fn transfer_tx(
            &self,
            token: AccountId,
            to: AccountId,
            amount: Balance,
        ) -> Result<(), PSP22Error> {
            PSP22Ref::transfer(&token, to, amount, vec![])?;

            Ok(())
        }

        /// Transfers a given amount of a PSP22 token on behalf of a specified account to another account
        ///
        /// Will revert if not enough allowance was given to the caller prior to executing this tx
        fn transfer_from_tx(
            &self,
            token: AccountId,
            from: AccountId,
            to: AccountId,
            amount: Balance,
        ) -> Result<(), DexError> {
            PSP22Ref::transfer_from_builder(&token, from, to, amount, vec![0x0])
                .call_flags(CallFlags::default().set_allow_reentry(true))
                .invoke()?;

            Ok(())
        }

        /// Returns the amount of unused allowance that the token owner has given to the spender
        fn allowance(&self, token: AccountId, owner: AccountId, spender: AccountId) -> Balance {
            PSP22Ref::allowance(&token, owner, spender)
        }

        /// Returns DEX balance of a PSP22 token for an account
        fn balance_of(&self, token: AccountId, account: AccountId) -> Balance {
            PSP22Ref::balance_of(&token, account)
        }

        fn check_role(&self, account: AccountId, role: Role) -> Result<(), DexError> {
            if self.access_control.has_role(account, role) {
                Ok(())
            } else {
                Err(DexError::MissingRole(account, role))
            }
        }

        fn emit_event<EE>(emitter: EE, event: Event)
        where
            EE: EmitEvent<SimpleDex>,
        {
            emitter.emit_event(event);
        }
    }

    impl Default for SimpleDex {
        fn default() -> Self {
            SimpleDex::new()
        }
    }

    #[cfg(test)]
    mod test {
        use proptest::prelude::*;

        use super::*;

        proptest! {
            #[test]
            fn rounding_benefits_dex(
                balance_token_a in 1..1000u128,
                balance_token_b in 1..1000u128,
                pay_token_a in 1..1000u128,
                fee_percentage in 0..10u128
            ) {
                let get_token_b =
                    SimpleDex::_out_given_in(pay_token_a, balance_token_a, balance_token_b, fee_percentage).unwrap();
                let balance_token_a = balance_token_a + pay_token_a;
                let balance_token_b = balance_token_b - get_token_b;
                let get_token_a =
                    SimpleDex::_out_given_in(get_token_b, balance_token_b, balance_token_a, fee_percentage).unwrap();

                assert!(get_token_a <= pay_token_a);
            }
        }
    }
}
