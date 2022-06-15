#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

/// This is the YellowButton
/// Rewards are distributed for extending the life of the button for as long as possible:
/// user_score = deadline - now
/// Pressiah gets 50% of tokens
/// the game is played until TheButton dies

#[ink::contract]
mod yellow_button {

    use button_token::{BALANCE_OF_SELECTOR, TRANSFER_SELECTOR};
    use ink_env::{
        call::{build_call, Call, ExecutionInput, Selector},
        DefaultEnvironment, Error as InkEnvError,
    };
    use ink_lang::{codegen::EmitEvent, reflect::ContractEventBase};
    use ink_prelude::{string::String, vec::Vec};
    use ink_storage::{traits::SpreadAllocate, Mapping};

    /// Error types
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Returned if given account already pressed The Button
        AlreadyParticipated,
        /// Returned if button is pressed after the deadline
        AfterDeadline,
        /// Account not whitelisted to play
        NotWhitelisted,
        /// Returned when an account which is not the owner calls a method with access control
        NotOwner,
        /// Returned if a call to another contract has failed
        ContractCall(String),
    }

    /// Result type
    pub type Result<T> = core::result::Result<T, Error>;
    /// Event type
    type Event = <YellowButton as ContractEventBase>::Type;

    impl From<InkEnvError> for Error {
        fn from(e: InkEnvError) -> Self {
            match e {
                InkEnvError::Decode(_e) => {
                    Error::ContractCall(String::from("Contract call failed due to Decode error"))
                }
                InkEnvError::CalleeTrapped => Error::ContractCall(String::from(
                    "Contract call failed due to CalleeTrapped error",
                )),
                InkEnvError::CalleeReverted => Error::ContractCall(String::from(
                    "Contract call failed due to CalleeReverted error",
                )),
                InkEnvError::KeyNotFound => Error::ContractCall(String::from(
                    "Contract call failed due to KeyNotFound error",
                )),
                InkEnvError::_BelowSubsistenceThreshold => Error::ContractCall(String::from(
                    "Contract call failed due to _BelowSubsistenceThreshold error",
                )),
                InkEnvError::TransferFailed => Error::ContractCall(String::from(
                    "Contract call failed due to TransferFailed error",
                )),
                InkEnvError::_EndowmentTooLow => Error::ContractCall(String::from(
                    "Contract call failed due to _EndowmentTooLow error",
                )),
                InkEnvError::CodeNotFound => Error::ContractCall(String::from(
                    "Contract call failed due to CodeNotFound error",
                )),
                InkEnvError::NotCallable => Error::ContractCall(String::from(
                    "Contract call failed due to NotCallable error",
                )),
                InkEnvError::Unknown => {
                    Error::ContractCall(String::from("Contract call failed due to Unknown error"))
                }
                InkEnvError::LoggingDisabled => Error::ContractCall(String::from(
                    "Contract call failed due to LoggingDisabled error",
                )),
                InkEnvError::EcdsaRecoveryFailed => Error::ContractCall(String::from(
                    "Contract call failed due to EcdsaRecoveryFailed error",
                )),
                #[cfg(any(feature = "std", test, doc))]
                InkEnvError::OffChain(_e) => {
                    Error::ContractCall(String::from("Contract call failed due to OffChain error"))
                }
            }
        }
    }

    /// Defines the storage
    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct YellowButton {
        /// access control
        owner: AccountId,
        /// How long does TheButton live for?
        button_lifetime: u32,
        /// is The Button dead
        is_dead: bool,
        // /// block number at which the game ends
        // deadline: u32,
        /// Stores a mapping between user accounts and the number of blocks they extended The Buttons life for
        presses: Mapping<AccountId, u32>,
        /// stores keys to `presses` because Mapping is not an Iterator. Heap-allocated! so we might need Map<u32, AccountId>
        press_accounts: Vec<AccountId>,
        /// stores total sum of user scores
        total_scores: u32,
        /// stores the last account that pressed The Button
        last_presser: Option<AccountId>,
        /// block number of the last press
        last_press: u32,
        /// AccountId of the ERC20 ButtonToken instance on-chain
        button_token: AccountId,
        /// accounts whitelisted to play the game
        can_play: Mapping<AccountId, bool>,
    }

    /// Event emitted when TheButton is pressed
    #[ink(event)]
    #[derive(Debug)]
    pub struct ButtonPressed {
        #[ink(topic)]
        by: AccountId,
        score: u32,
        total_scores: u32,
        when: u32,
        previous_press: u32,
        new_deadline: u32,
    }

    /// Event emitted when TheButton owner is changed
    #[ink(event)]
    #[derive(Debug)]
    pub struct OwnershipTransferred {
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
    }

    /// Event emitted when TheButton is created
    #[ink(event)]
    #[derive(Debug)]
    pub struct ButtonCreated {
        #[ink(topic)]
        button_token: AccountId,
        start: u32,
        deadline: u32,
    }

    /// Event emitted when account is whitelisted to play the game
    #[ink(event)]
    #[derive(Debug)]
    pub struct AccountWhitelisted {
        #[ink(topic)]
        player: AccountId,
    }

    /// Even emitted when button death is triggered    
    #[ink(event)]
    #[derive(Debug)]
    pub struct ButtonDeath {
        #[ink(topic)]
        pressiah: Option<AccountId>,
        pressiah_reward: u128,
        rewards: Vec<(AccountId, u128)>,
    }

    impl YellowButton {
        /// Returns the buttons status
        #[ink(message)]
        pub fn is_dead(&self) -> bool {
            self.is_dead
        }

        /// Returns the current deadline
        #[ink(message)]
        pub fn deadline(&self) -> u32 {
            self.last_press + self.button_lifetime
        }

        /// Returns the user score
        #[ink(message)]
        pub fn score_of(&self, user: AccountId) -> u32 {
            self.presses.get(&user).unwrap_or(0)
        }

        /// Returns whether given account can play
        #[ink(message)]
        pub fn can_play(&self, user: AccountId) -> bool {
            self.can_play.get(&user).unwrap_or(false)
        }

        /// Returns the account id that pressed as last
        #[ink(message)]
        pub fn last_presser(&self) -> Option<AccountId> {
            self.last_presser
        }

        /// Returns address of the game's token
        #[ink(message)]
        pub fn get_button_token(&self) -> Result<AccountId> {
            Ok(self.button_token)
        }

        /// Returns game token balance of the game contract
        #[ink(message)]
        pub fn get_balance(&self) -> Result<Balance> {
            let this = self.env().account_id();
            let button_token = self.button_token;

            let balance = build_call::<DefaultEnvironment>()
                .call_type(Call::new().callee(button_token))
                .exec_input(ExecutionInput::new(Selector::new(BALANCE_OF_SELECTOR)).push_arg(this))
                .returns::<Balance>()
                .fire()?;

            Ok(balance)
        }

        fn emit_event<EE: EmitEvent<YellowButton>>(emitter: EE, event: Event) {
            emitter.emit_event(event);
        }

        /// Constructor
        #[ink(constructor)]
        pub fn new(button_token: AccountId, button_lifetime: u32) -> Self {
            ink_lang::utils::initialize_contract(|contract: &mut Self| {
                let now = Self::env().block_number();
                let caller = Self::env().caller();
                let deadline = now + button_lifetime;

                contract.owner = caller;
                contract.is_dead = false;
                contract.last_press = now;
                contract.button_lifetime = button_lifetime;
                // contract.deadline = deadline;
                contract.button_token = button_token;

                let event = Event::ButtonCreated(ButtonCreated {
                    start: now,
                    deadline,
                    button_token,
                });

                Self::emit_event(Self::env(), event)
            })
        }

        fn transfer_tx(&self, to: AccountId, value: u128) -> core::result::Result<(), InkEnvError> {
            build_call::<DefaultEnvironment>()
                .call_type(Call::new().callee(self.button_token))
                .exec_input(
                    ExecutionInput::new(Selector::new(TRANSFER_SELECTOR))
                        .push_arg(to)
                        .push_arg(value),
                )
                .returns::<()>()
                .fire()
        }

        /// End of the game logic
        ///
        /// distributes the rewards to the participants
        fn death(&mut self) -> Result<()> {
            self.is_dead = true;

            let total_balance = Self::get_balance(self)?;

            // Pressiah gets 50% of supply
            let pressiah_reward = total_balance / 2;
            if let Some(pressiah) = self.last_presser {
                self.transfer_tx(pressiah, pressiah_reward)?;
            }

            let total = self.total_scores;
            let remaining_balance = total_balance - pressiah_reward;
            let mut rewards = Vec::new();
            // rewards are distributed to participants proportionally to their score
            let _ = self
                .press_accounts
                .iter()
                .try_for_each(|account_id| -> Result<()> {
                    if let Some(score) = self.presses.get(account_id) {
                        let reward = (score as u128 * remaining_balance) / total as u128;
                        rewards.push((*account_id, reward));
                        // transfer amount
                        return Ok(self.transfer_tx(*account_id, reward)?);
                    }
                    Ok(())
                });

            let event = Event::ButtonDeath(ButtonDeath {
                pressiah: self.last_presser,
                pressiah_reward,
                rewards,
            });

            Self::emit_event(self.env(), event);
            Ok(())
        }

        /// Whitelists given AccountId to participate in the game
        ///
        /// returns an error if called by someone else but the owner
        #[ink(message)]
        pub fn allow(&mut self, player: AccountId) -> Result<()> {
            if Self::env().caller() != self.owner {
                return Err(Error::NotOwner);
            }

            self.can_play.insert(player, &true);
            let event = Event::AccountWhitelisted(AccountWhitelisted { player });
            Self::emit_event(self.env(), event);
            Ok(())
        }

        /// Whitelists an array of accounts to participate in the game
        ///
        /// returns an error if called by someone else but the owner
        #[ink(message)]
        pub fn bulk_allow(&mut self, players: Vec<AccountId>) -> Result<()> {
            if Self::env().caller() != self.owner {
                return Err(Error::NotOwner);
            }

            for player in players {
                Self::allow(self, player)?;
            }

            Ok(())
        }

        /// Blacklists given AccountId from participating in the game
        ///
        /// returns an error if called by someone else but the owner
        #[ink(message)]
        pub fn disallow(&mut self, player: AccountId) -> Result<()> {
            let caller = Self::env().caller();
            if caller != self.owner {
                return Err(Error::NotOwner);
            }
            self.can_play.insert(player, &false);
            Ok(())
        }

        /// Terminates the contract
        ///
        /// can only be called by the contract owner
        #[ink(message)]
        pub fn terminate(&mut self) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::NotOwner);
            }

            self.env().terminate_contract(caller)
        }

        /// Transfers ownership of the contract to a new account
        ///
        /// Can only be called by the current owner
        #[ink(message)]
        pub fn transfer_ownership(&mut self, to: AccountId) -> Result<()> {
            let caller = Self::env().caller();
            if caller != self.owner {
                return Err(Error::NotOwner);
            }
            self.owner = to;

            let event = Event::OwnershipTransferred(OwnershipTransferred { from: caller, to });
            Self::emit_event(self.env(), event);

            Ok(())
        }

        /// Button press logic
        #[ink(message)]
        pub fn press(&mut self) -> Result<()> {
            if self.is_dead {
                return Err(Error::AfterDeadline);
            }

            let now = self.env().block_number();
            if now > self.deadline() {
                // trigger TheButton's death
                // at this point is is after the deadline but the death event has not yet been triggered
                // to distribute the awards
                // the last account to click the button in this state will pay for all the computations
                // but that should be OK (similar to paying for distributing staking rewards)
                return self.death();
            }

            let caller = self.env().caller();
            if self.presses.get(&caller).is_some() {
                return Err(Error::AlreadyParticipated);
            }

            if !self.can_play.get(&caller).unwrap_or(false) {
                return Err(Error::NotWhitelisted);
            }

            // record press
            // score is the number of blocks the button life was extended for
            // this incentivizes pressing as late as possible in the game (but not too late)
            let previous_press = self.last_press;
            let score = now - previous_press;
            let new_deadline = now + self.button_lifetime;
            self.presses.insert(&caller, &score);
            self.press_accounts.push(caller);
            self.last_presser = Some(caller);
            self.last_press = now;
            self.total_scores += score;

            // emit event
            let event = Event::ButtonPressed(ButtonPressed {
                by: caller,
                previous_press,
                score,
                when: now,
                new_deadline,
                total_scores: self.total_scores,
            });
            Self::emit_event(self.env(), event);

            Ok(())
        }
    }

    // TODO : what can we test here actually?
    // TODO : just pressing I suppose
    #[cfg(test)]
    mod tests {
        use super::*;
        use button_token::{ButtonToken, Event as ButtonTokenEvent};
        use ink_lang as ink;

        #[ink::test]
        fn play_the_game() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();

            let alice = accounts.alice;
            let bob = accounts.bob;
            let charlie = accounts.charlie;

            let button_token_address = accounts.frank; //AccountId::from([0xFA; 32]);
            let game_address = accounts.django; //AccountId::from([0xF9; 32]);

            // alice deploys the token contract
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(alice);
            ink_env::test::set_callee::<ink_env::DefaultEnvironment>(button_token_address);
            let mut button_token = ButtonToken::new(1000);

            // alice deploys the game contract
            let button_lifetime = 3;
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(alice);
            ink_env::test::set_callee::<ink_env::DefaultEnvironment>(game_address);
            let mut game = YellowButton::new(button_token_address, button_lifetime);

            let emitted_events = ink_env::test::recorded_events().collect::<Vec<_>>();
            let button_created_event = &emitted_events[1];
            let decoded_event: Event =
                <Event as scale::Decode>::decode(&mut &button_created_event.data[..])
                    .expect("Can't decode as Event");

            match decoded_event {
                Event::ButtonCreated(ButtonCreated {
                    start,
                    deadline,
                    button_token,
                }) => {
                    assert_eq!(deadline, button_lifetime, "Wrong ButtonCreated.deadline");
                    assert_eq!(start, 0, "Wrong ButtonCreated.start");
                    assert_eq!(
                        button_token, button_token_address,
                        "Wrong ButtonCreated.button_token"
                    );
                }
                _ => panic!("Wrong event emitted"),
            }

            // Alice transfer all token balance to the game
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(alice);
            ink_env::test::set_callee::<ink_env::DefaultEnvironment>(button_token_address);

            assert!(
                button_token.transfer(game_address, 999).is_ok(),
                "Transfer call failed"
            );

            let emitted_events = ink_env::test::recorded_events().collect::<Vec<_>>();
            let transfer_event = &emitted_events[2];
            let decoded_event: ButtonTokenEvent =
                <ButtonTokenEvent as scale::Decode>::decode(&mut &transfer_event.data[..])
                    .expect("Can't decode as Event");

            match decoded_event {
                ButtonTokenEvent::Transfer(event) => {
                    assert_eq!(event.value, 999, "Wrong Transfer.value");
                    assert_eq!(event.from, Some(alice), "Wrong Transfer.from");
                    assert_eq!(event.to, Some(game_address), "Wrong Transfer.from");
                }
                _ => panic!("Wrong event emitted"),
            }

            // Alice is the owner and whitelists accounts for playing
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(alice);
            ink_env::test::set_callee::<ink_env::DefaultEnvironment>(game_address);
            assert!(
                game.bulk_allow(vec![bob, charlie]).is_ok(),
                "Bulk allow call failed"
            );

            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(bob);
            ink_env::test::set_callee::<ink_env::DefaultEnvironment>(game_address);
            assert!(game.press().is_ok(), "Press call failed");

            ink_env::test::advance_block::<ink_env::DefaultEnvironment>();

            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(charlie);
            ink_env::test::set_callee::<ink_env::DefaultEnvironment>(game_address);
            assert!(game.press().is_ok(), "Press call failed");

            // NOTE : we cannot test reward distribution, cross-contract calls are not yet supported in the test environment
        }
    }
}
