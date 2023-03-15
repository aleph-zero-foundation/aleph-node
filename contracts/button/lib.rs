#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::let_unit_value)]

mod errors;

#[ink::contract]
pub mod button_game {
    use access_control::{roles::Role, AccessControlRef, ACCESS_CONTROL_PUBKEY};
    #[cfg(feature = "std")]
    use ink::storage::traits::StorageLayout;
    use ink::{
        codegen::EmitEvent,
        env::{call::FromAccountId, CallFlags},
        prelude::vec,
        reflect::ContractEventBase,
        ToAccountId,
    };
    use marketplace::marketplace::MarketplaceRef;
    use openbrush::contracts::psp22::{extensions::mintable::PSP22MintableRef, PSP22Ref};
    use scale::{Decode, Encode};

    use crate::errors::GameError;

    /// Result type
    type ButtonResult<T> = core::result::Result<T, GameError>;

    /// Event type
    type Event = <ButtonGame as ContractEventBase>::Type;

    /// Event emitted when TheButton is created
    #[ink(event)]
    #[derive(Debug)]
    pub struct ButtonCreated {
        #[ink(topic)]
        reward_token: AccountId,
        #[ink(topic)]
        ticket_token: AccountId,
        start: BlockNumber,
        deadline: BlockNumber,
    }

    /// Event emitted when TheButton is pressed
    #[ink(event)]
    #[derive(Debug)]
    pub struct ButtonPressed {
        #[ink(topic)]
        by: AccountId,
        when: BlockNumber,
        score: Balance,
    }

    /// Event emitted when the finished game is reset and pressiah is rewarded
    #[ink(event)]
    #[derive(Debug)]
    pub struct GameReset {
        when: BlockNumber,
    }

    /// Scoring strategy indicating what kind of reward users get for pressing the button
    #[derive(Debug, Encode, Decode, Clone, Copy, PartialEq, Eq)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, StorageLayout))]
    pub enum Scoring {
        /// Pressing the button as soon as possible gives the highest reward
        EarlyBirdSpecial,
        /// Pressing the button as late as possible gives the highest reward
        BackToTheFuture,
        /// The reward increases linearly with the number of participants
        ThePressiahCometh,
    }

    /// Game contracts storage
    #[ink(storage)]
    pub struct ButtonGame {
        /// How long does TheButton live for?
        pub button_lifetime: BlockNumber,
        /// stores the last account that pressed The Button
        pub last_presser: Option<AccountId>,
        /// block number of the last press, set to current block number at button start/reset
        pub last_press: BlockNumber,
        /// sum of rewards paid to players in the current iteration
        pub total_rewards: u128,
        /// counter for the number of presses
        pub presses: u128,
        /// AccountId of the PSP22 ButtonToken instance on-chain
        pub reward_token: AccountId,
        /// Account ID of the ticket token
        pub ticket_token: AccountId,
        /// access control contract
        pub access_control: AccessControlRef,
        /// ticket marketplace contract
        pub marketplace: MarketplaceRef,
        /// scoring strategy
        pub scoring: Scoring,
        /// current round number
        pub round: u64,
    }

    impl ButtonGame {
        #[ink(constructor)]
        pub fn new(
            ticket_token: AccountId,
            reward_token: AccountId,
            marketplace: AccountId,
            button_lifetime: BlockNumber,
            scoring: Scoring,
        ) -> Self {
            let caller = Self::env().caller();
            let code_hash = Self::env()
                .own_code_hash()
                .expect("Called new on a contract with no code hash");
            let required_role = Role::Initializer(code_hash);
            let access_control = AccountId::from(ACCESS_CONTROL_PUBKEY);
            let access_control = AccessControlRef::from_account_id(access_control);

            match ButtonGame::check_role(&access_control, caller, required_role) {
                Ok(_) => Self::init(
                    access_control,
                    ticket_token,
                    reward_token,
                    marketplace,
                    button_lifetime,
                    scoring,
                ),
                Err(why) => panic!("Could not initialize the contract {:?}", why),
            }
        }

        /// Returns the current deadline
        ///
        /// Deadline is the block number at which the game will end if there are no more participants
        #[ink(message)]
        pub fn deadline(&self) -> BlockNumber {
            self.last_press + self.button_lifetime
        }

        /// Returns the curent round number
        #[ink(message)]
        pub fn round(&self) -> u64 {
            self.round
        }

        /// Returns the buttons status
        #[ink(message)]
        pub fn is_dead(&self) -> bool {
            self.env().block_number() > self.deadline()
        }

        /// Returns the last player who pressed the button.
        /// If button is dead, this is The Pressiah.
        #[ink(message)]
        pub fn last_presser(&self) -> Option<AccountId> {
            self.last_presser
        }

        /// Returns the current access control contract address
        #[ink(message)]
        pub fn access_control(&self) -> AccountId {
            self.access_control.to_account_id()
        }

        /// Returns address of the game's reward token
        #[ink(message)]
        pub fn reward_token(&self) -> AccountId {
            self.reward_token
        }

        /// Returns address of the game's ticket token
        #[ink(message)]
        pub fn ticket_token(&self) -> AccountId {
            self.ticket_token
        }

        /// Returns the address of the marketplace for exchanging this game's rewards for tickets.
        #[ink(message)]
        pub fn marketplace(&self) -> AccountId {
            self.marketplace.to_account_id()
        }

        /// Returns own code hash
        #[ink(message)]
        pub fn code_hash(&self) -> ButtonResult<Hash> {
            self.env()
                .own_code_hash()
                .map_err(|_| GameError::CantRetrieveOwnCodeHash)
        }

        /// Presses the button
        ///
        /// If called on alive button, instantaneously mints reward tokens to the caller
        #[ink(message)]
        pub fn press(&mut self) -> ButtonResult<()> {
            if self.is_dead() {
                return Err(GameError::AfterDeadline);
            }

            let caller = self.env().caller();
            let now = Self::env().block_number();
            let this = self.env().account_id();

            // transfers 1 ticket token from the caller to self
            // tx will fail if user did not give allowance to the game contract
            // or does not have enough balance
            self.transfer_ticket(caller, this, 1u128)?;

            let score = self.score(now);

            // mints reward tokens to pay out the reward
            // contract needs to have a Minter role on the reward token contract
            self.mint_reward(caller, score)?;

            self.presses += 1;
            self.last_presser = Some(caller);
            self.last_press = now;
            self.total_rewards += score;

            Self::emit_event(
                self.env(),
                Event::ButtonPressed(ButtonPressed {
                    by: caller,
                    when: now,
                    score,
                }),
            );

            Ok(())
        }

        /// Resets the game
        ///
        /// Erases the storage and pays award to the Pressiah
        /// Can be called by any account on behalf of a player
        /// Can only be called after button's deadline
        #[ink(message)]
        pub fn reset(&mut self) -> ButtonResult<()> {
            self.ensure_dead()?;
            self.reward_pressiah()?;
            self.reset_state()?;
            self.transfer_tickets_to_marketplace()?;
            self.reset_marketplace()
        }

        /// Sets new access control contract address
        ///
        /// Should only be called by the contract owner
        /// Implementing contract is responsible for setting up proper AccessControl
        #[ink(message)]
        pub fn set_access_control(&mut self, new_access_control: AccountId) -> ButtonResult<()> {
            let caller = self.env().caller();
            let this = self.env().account_id();
            let required_role = Role::Owner(this);
            ButtonGame::check_role(&self.access_control, caller, required_role)?;
            self.access_control = AccessControlRef::from_account_id(new_access_control);
            Ok(())
        }

        /// Terminates the contract
        ///
        /// Should only be called by the contract Owner
        #[ink(message)]
        pub fn terminate(&mut self) -> ButtonResult<()> {
            let caller = self.env().caller();
            let this = self.env().account_id();
            let required_role = Role::Owner(this);
            ButtonGame::check_role(&self.access_control, caller, required_role)?;
            self.env().terminate_contract(caller)
        }

        //===================================================================================================

        fn init(
            access_control: AccessControlRef,
            ticket_token: AccountId,
            reward_token: AccountId,
            marketplace: AccountId,
            button_lifetime: BlockNumber,
            scoring: Scoring,
        ) -> Self {
            let now = Self::env().block_number();
            let deadline = now + button_lifetime;

            let contract = Self {
                access_control,
                button_lifetime,
                reward_token,
                ticket_token,
                marketplace: MarketplaceRef::from_account_id(marketplace),
                last_press: now,
                scoring,
                last_presser: None,
                presses: 0,
                total_rewards: 0,
                round: 0,
            };

            Self::emit_event(
                Self::env(),
                Event::ButtonCreated(ButtonCreated {
                    start: now,
                    deadline,
                    ticket_token,
                    reward_token,
                }),
            );

            contract
        }

        fn reset_state(&mut self) -> ButtonResult<()> {
            let now = self.env().block_number();

            self.presses = 0;
            self.last_presser = None;
            self.last_press = now;
            self.total_rewards = 0;
            self.round.checked_add(1).ok_or(GameError::Arithmethic)?;

            Self::emit_event(self.env(), Event::GameReset(GameReset { when: now }));
            Ok(())
        }

        fn reward_pressiah(&self) -> ButtonResult<()> {
            if let Some(pressiah) = self.last_presser {
                let reward = self.pressiah_score();
                self.mint_reward(pressiah, reward)?;
            };

            Ok(())
        }

        fn ensure_dead(&self) -> ButtonResult<()> {
            if !self.is_dead() {
                Err(GameError::BeforeDeadline)
            } else {
                Ok(())
            }
        }

        fn transfer_tickets_to_marketplace(&self) -> ButtonResult<()> {
            PSP22Ref::transfer_builder(
                &self.ticket_token,
                self.marketplace.to_account_id(),
                self.held_tickets(),
                vec![],
            )
            .call_flags(CallFlags::default().set_allow_reentry(true))
            .invoke()?;

            Ok(())
        }

        fn held_tickets(&self) -> Balance {
            PSP22Ref::balance_of(&self.ticket_token, self.env().account_id())
        }

        fn reset_marketplace(&mut self) -> ButtonResult<()> {
            self.marketplace.reset()?;

            Ok(())
        }

        fn check_role(
            access_control: &AccessControlRef,
            account: AccountId,
            role: Role,
        ) -> ButtonResult<()> {
            if access_control.has_role(account, role) {
                Ok(())
            } else {
                Err(GameError::MissingRole(role))
            }
        }

        fn score(&self, now: BlockNumber) -> Balance {
            match self.scoring {
                Scoring::EarlyBirdSpecial => self.deadline().saturating_sub(now) as Balance,
                Scoring::BackToTheFuture => now.saturating_sub(self.last_press) as Balance,
                Scoring::ThePressiahCometh => (self.presses + 1) as Balance,
            }
        }

        fn pressiah_score(&self) -> Balance {
            (self.total_rewards / 4) as Balance
        }

        fn transfer_ticket(
            &self,
            from: AccountId,
            to: AccountId,
            value: Balance,
        ) -> ButtonResult<()> {
            PSP22Ref::transfer_from_builder(&self.ticket_token, from, to, value, vec![])
                .call_flags(CallFlags::default().set_allow_reentry(true))
                .invoke()?;

            Ok(())
        }

        fn mint_reward(&self, to: AccountId, amount: Balance) -> ButtonResult<()> {
            PSP22MintableRef::mint(&self.reward_token, to, amount)?;

            Ok(())
        }

        fn emit_event<EE>(emitter: EE, event: Event)
        where
            EE: EmitEvent<ButtonGame>,
        {
            emitter.emit_event(event);
        }
    }
}
