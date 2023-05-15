//! Implements a Dutch auction of one token for another.
//!
//! This contract will auction off units of one token (referred to as `tickets`), accepting payment
//! in another token (`reward token`). The auction keeps track of the average price over all sales
//! made (before the first auction, this number is set on contract initialization) and starts off
//! the auction at `average_price() * sale_multiplier()`. Afterwards the price decreases linearly
//! with each block until reaching `min_price()` after `auction_length()` blocks, at which point
//! the price stays at that level indefinitely.
//!
//! A user can use the `buy(max_price)` call (after issuing a `psp22::approve` for the appropriate
//! amount to the reward token contract) to accept the current price and buy one ticket. This
//! transaction will fail if the price increased to above `max_price`, for example, due to a ticket
//! getting sold.
//!
//! The admin of the contract is expected to transfer a number of reward tokens for sale
//! into this contract and then call `reset()` in the same transaction to begin the auction. Calling
//! `reset()` if an auction is already in progress.

#![cfg_attr(not(feature = "std"), no_std)]
#![feature(min_specialization)]
#![allow(clippy::let_unit_value)]

pub const RESET_SELECTOR: [u8; 4] = [0x00, 0x00, 0x00, 0x01];

#[ink::contract]
pub mod marketplace {
    use access_control::{roles::Role, AccessControlRef, ACCESS_CONTROL_PUBKEY};
    use ink::{
        codegen::{EmitEvent, Env},
        env::{
            call::{build_call, ExecutionInput, FromAccountId},
            set_code_hash, DefaultEnvironment,
        },
        prelude::{format, string::String, vec},
        reflect::ContractEventBase,
        storage::{traits::ManualKey, Lazy},
        LangError,
    };
    use openbrush::{
        contracts::psp22::{extensions::burnable::PSP22BurnableRef, PSP22Error, PSP22Ref},
        traits::Storage,
    };
    use shared_traits::{Haltable, HaltableData, HaltableError, Internal, Selector};

    type Event = <Marketplace as ContractEventBase>::Type;

    #[derive(Debug)]
    #[ink::storage_item]
    pub struct Data {
        total_proceeds: Balance,
        tickets_sold: Balance,
        min_price: Balance,
        current_start_block: BlockNumber,
        auction_length: BlockNumber,
        sale_multiplier: Balance,
        ticket_token: AccountId,
        reward_token: AccountId,
        access_control: AccessControlRef,
    }

    #[ink(storage)]
    #[derive(Storage)]
    pub struct Marketplace {
        pub data: Lazy<Data, ManualKey<0x44415441>>,
        #[storage_field]
        pub halted: HaltableData,
    }

    #[derive(Eq, PartialEq, Debug, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        HaltableError(HaltableError),
        MissingRole(Role),
        ContractCall(String),
        PSP22TokenCall(PSP22Error),
        MaxPriceExceeded,
        MarketplaceEmpty,
    }

    #[ink(event)]
    pub struct Halted;

    #[ink(event)]
    pub struct Resumed;

    #[ink(event)]
    #[derive(Clone, Eq, PartialEq, Debug)]
    pub struct TicketBought {
        #[ink(topic)]
        pub ticket: AccountId,
        #[ink(topic)]
        pub by: AccountId,
        pub price: Balance,
    }

    #[ink(event)]
    #[derive(Clone, Eq, PartialEq, Debug)]
    pub struct MarketplaceReset;

    impl From<ink::env::Error> for Error {
        fn from(inner: ink::env::Error) -> Self {
            Error::ContractCall(format!("{:?}", inner))
        }
    }

    impl From<PSP22Error> for Error {
        fn from(inner: PSP22Error) -> Self {
            Error::PSP22TokenCall(inner)
        }
    }

    impl From<LangError> for Error {
        fn from(inner: LangError) -> Self {
            Error::ContractCall(format!("{:?}", inner))
        }
    }

    impl From<Error> for HaltableError {
        fn from(inner: Error) -> Self {
            HaltableError::Custom(format!("{:?}", inner))
        }
    }

    impl From<HaltableError> for Error {
        fn from(inner: HaltableError) -> Self {
            Error::HaltableError(inner)
        }
    }

    impl Internal for Marketplace {
        fn _after_halt(&self) -> Result<(), HaltableError> {
            Self::emit_event(self.env(), Event::Halted(Halted {}));
            Ok(())
        }

        fn _after_resume(&self) -> Result<(), HaltableError> {
            Self::emit_event(self.env(), Event::Resumed(Resumed {}));
            Ok(())
        }

        fn _before_halt(&self) -> Result<(), HaltableError> {
            self.check_role(self.env().caller(), Role::Admin(self.env().account_id()))?;
            Ok(())
        }

        fn _before_resume(&self) -> Result<(), HaltableError> {
            self.check_role(self.env().caller(), Role::Admin(self.env().account_id()))?;
            Ok(())
        }
    }

    impl Haltable for Marketplace {}

    impl Marketplace {
        #[ink(constructor)]
        pub fn new(
            ticket_token: AccountId,
            reward_token: AccountId,
            starting_price: Balance,
            min_price: Balance,
            sale_multiplier: Balance,
            auction_length: BlockNumber,
        ) -> Self {
            let access_control = AccountId::from(ACCESS_CONTROL_PUBKEY);
            let access_control = AccessControlRef::from_account_id(access_control);
            if access_control.has_role(Self::env().caller(), Self::initializer()) {
                let mut data = Lazy::new();
                data.set(&Data {
                    ticket_token,
                    reward_token,
                    min_price,
                    sale_multiplier,
                    auction_length,
                    current_start_block: Self::env().block_number(),
                    total_proceeds: starting_price.saturating_div(sale_multiplier),
                    tickets_sold: 1,
                    access_control,
                });

                Marketplace {
                    data,
                    halted: HaltableData {
                        halted: Lazy::new(),
                    },
                }
            } else {
                panic!("Caller is not allowed to initialize this contract");
            }
        }

        /// The length of each auction of a single ticket in blocks.
        ///
        /// The contract will decrease the price linearly from `average_price() * sale_multiplier()`
        /// to `min_price()` over this period. The auction doesn't end after the period elapses -
        /// the ticket remains available for purchase at `min_price()`.
        #[ink(message)]
        pub fn auction_length(&self) -> BlockNumber {
            self.data.get().unwrap().auction_length
        }

        /// The block at which the auction of the current ticket started.
        #[ink(message)]
        pub fn current_start_block(&self) -> BlockNumber {
            self.data.get().unwrap().current_start_block
        }

        /// The price the contract would charge when buying at the current block.
        #[ink(message)]
        pub fn price(&self) -> Balance {
            self.current_price()
        }

        /// The average price over all sales the contract made.
        #[ink(message)]
        pub fn average_price(&self) -> Balance {
            let data = self.data.get().unwrap();
            Self::_average_price(&data)
        }

        pub fn _average_price(data: &Data) -> Balance {
            data.total_proceeds.saturating_div(data.tickets_sold)
        }

        /// The multiplier applied to the average price after each sale.
        ///
        /// The contract tracks the average price of all sold tickets and starts off each new
        /// auction at `price() = average_price() * sale_multiplier()`.
        #[ink(message)]
        pub fn sale_multiplier(&self) -> Balance {
            self.data.get().unwrap().sale_multiplier
        }

        /// Set the value of the multiplier applied to the average price after each sale.
        ///
        /// Requires `Role::Admin`.
        #[ink(message)]
        pub fn set_sale_multiplier(&mut self, sale_multiplier: Balance) -> Result<(), Error> {
            self.check_role(Self::env().caller(), self.admin())?;

            let mut data = self.data.get().unwrap();
            data.sale_multiplier = sale_multiplier;
            self.data.set(&data);

            Ok(())
        }

        /// Number of tickets available for sale.
        ///
        /// The tickets will be auctioned off one by one.
        #[ink(message)]
        pub fn available_tickets(&self) -> Balance {
            self.ticket_balance()
        }

        /// The minimal price the contract allows.
        #[ink(message)]
        pub fn min_price(&self) -> Balance {
            self.data.get().unwrap().min_price
        }

        /// Update the minimal price.
        ///
        /// Requires `Role::Admin`.
        #[ink(message)]
        pub fn set_min_price(&mut self, value: Balance) -> Result<(), Error> {
            self.check_role(Self::env().caller(), self.admin())?;

            let mut data = self.data.get().unwrap();
            data.min_price = value;
            self.data.set(&data);

            Ok(())
        }

        /// Update the length of the auction.
        ///
        /// Requires `Role::Admin`.
        #[ink(message)]
        pub fn set_auction_length(&mut self, new_auction_length: BlockNumber) -> Result<(), Error> {
            self.check_role(self.env().caller(), self.admin())?;

            let mut data = self.data.get().unwrap();
            data.auction_length = new_auction_length;
            self.data.set(&data);

            Ok(())
        }

        /// Address of the reward token contract this contract will accept as payment.
        #[ink(message)]
        pub fn reward_token(&self) -> AccountId {
            self.data.get().unwrap().reward_token
        }

        /// Address of the ticket token contract this contract will auction off.
        #[ink(message)]
        pub fn ticket_token(&self) -> AccountId {
            self.data.get().unwrap().ticket_token
        }

        /// Buy one ticket at the current_price.
        ///
        /// The caller should make an approval for at least `price()` reward tokens to make sure the
        /// call will succeed. The caller can specify a `max_price` - the call will fail if the
        /// current price is greater than that.
        #[ink(message)]
        pub fn buy(&mut self, max_price: Option<Balance>) -> Result<(), Error> {
            self.check_halted()?;

            if self.ticket_balance() == 0 {
                return Err(Error::MarketplaceEmpty);
            }

            let price = self.current_price();
            if let Some(max_price) = max_price {
                if price > max_price {
                    return Err(Error::MaxPriceExceeded);
                }
            }

            let caller = self.env().caller();

            self.take_payment(caller, price)?;
            self.give_ticket(caller)?;

            let mut data = self.data.get().unwrap();

            data.total_proceeds = data.total_proceeds.saturating_add(price);
            data.tickets_sold = data.tickets_sold.saturating_add(1);
            data.current_start_block = self.env().block_number();

            self.data.set(&data);

            Self::emit_event(
                self.env(),
                Event::TicketBought(TicketBought {
                    ticket: data.ticket_token,
                    price,
                    by: caller,
                }),
            );

            Ok(())
        }

        /// Re-start the auction from the current block.
        /// Note that this will keep the average estimate from previous auctions.
        ///
        /// Requires `Role::Admin`.
        #[ink(message, selector = 0x00000001)]
        pub fn reset(&mut self) -> Result<(), Error> {
            self.check_role(self.env().caller(), self.admin())?;

            let mut data = self.data.get().unwrap();
            data.current_start_block = self.env().block_number();
            self.data.set(&data);

            Self::emit_event(self.env(), Event::MarketplaceReset(MarketplaceReset {}));

            Ok(())
        }

        /// Terminates the contract
        ///
        /// Requires `Role::Admin`.
        #[ink(message)]
        pub fn terminate(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();
            self.check_role(caller, self.admin())?;
            self.env().terminate_contract(caller)
        }

        /// Upgrades contract code
        ///
        /// Requires `Role::Admin`.
        #[ink(message)]
        pub fn set_code(
            &mut self,
            code_hash: [u8; 32],
            callback: Option<Selector>,
        ) -> Result<(), Error> {
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
                    .returns::<Result<(), Error>>()
                    .invoke()?;
            }

            Ok(())
        }

        fn current_price(&self) -> Balance {
            let data = self.data.get().unwrap();
            linear_decrease(
                data.current_start_block.into(),
                Self::_average_price(&data).saturating_mul(data.sale_multiplier),
                data.current_start_block
                    .saturating_add(data.auction_length)
                    .into(),
                data.min_price,
                self.env().block_number().into(),
            )
            .max(data.min_price)
        }

        fn take_payment(&self, from: AccountId, amount: Balance) -> Result<(), Error> {
            PSP22BurnableRef::burn_builder(&self.data.get().unwrap().reward_token, from, amount)
                .call_flags(ink::env::CallFlags::default().set_allow_reentry(true))
                .invoke()?;

            Ok(())
        }

        fn give_ticket(&self, to: AccountId) -> Result<(), Error> {
            PSP22Ref::transfer(&self.data.get().unwrap().ticket_token, to, 1, vec![])?;

            Ok(())
        }

        fn ticket_balance(&self) -> Balance {
            PSP22Ref::balance_of(
                &self.data.get().unwrap().ticket_token,
                self.env().account_id(),
            )
        }

        fn check_role(&self, account: AccountId, role: Role) -> Result<(), Error> {
            if self
                .data
                .get()
                .unwrap()
                .access_control
                .has_role(account, role)
            {
                Ok(())
            } else {
                Err(Error::MissingRole(role))
            }
        }

        fn initializer() -> Role {
            let code_hash = Self::env()
                .own_code_hash()
                .expect("Failure to retrieve code hash.");
            Role::Initializer(code_hash)
        }

        fn admin(&self) -> Role {
            Role::Admin(self.env().account_id())
        }

        fn emit_event<EE: EmitEvent<Self>>(emitter: EE, event: Event) {
            emitter.emit_event(event)
        }
    }

    /// Returns (an approximation of) the linear function passing through `(x_start, y_start)` and `(x_end, y_end)` at
    /// `x`. If `x` is outside the range of `x_start` and `x_end`, the value of `y` at the closest endpoint is returned.
    fn linear_decrease(x_start: u128, y_start: u128, x_end: u128, y_end: u128, x: u128) -> u128 {
        let steps = x.saturating_sub(x_start);
        let x_span = x_end.saturating_sub(x_start);
        let y_span = y_start.saturating_sub(y_end);

        if x >= x_end {
            y_end
        } else if x <= x_start {
            y_start
        } else if y_span > x_span {
            let y_per_x = y_span.saturating_div(x_span);
            y_start.saturating_sub(steps.saturating_mul(y_per_x))
        } else {
            let x_per_y = x_span.saturating_div(y_span);
            y_start.saturating_sub(steps.saturating_div(x_per_y))
        }
    }

    #[cfg(test)]
    mod test {
        use assert2::assert;

        use super::*;

        #[test]
        fn test_linear_decrease_with_slope_over_1() {
            assert!(linear_decrease(1, 100, 50, 1, 2) == 98);
            assert!(linear_decrease(1, 100, 50, 1, 3) == 96);
        }

        #[ink::test]
        fn test_linear_decrease_with_slope_under_1() {
            assert!(linear_decrease(1, 50, 100, 1, 2) == 50);
            assert!(linear_decrease(1, 50, 100, 1, 3) == 49);
        }

        #[ink::test]
        fn test_linear_decrease_with_slope_equal_1() {
            assert!(linear_decrease(1, 50, 50, 1, 2) == 49);
            assert!(linear_decrease(1, 50, 50, 1, 3) == 48);
        }
    }
}
