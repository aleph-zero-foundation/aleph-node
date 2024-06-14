#![allow(clippy::nonminimal_bool)]

use frame_support::dispatch::DispatchResult;
use parity_scale_codec::Encode;
use sp_core::hexdisplay::HexDisplay;
use sp_runtime::DispatchError;

use crate::{
    pallet::{Config, Event, Pallet},
    traits::{
        AccountInfoProvider, BalancesProvider, BondedStashProvider, ContractInfoProvider,
        NextKeysSessionProvider,
    },
    LOG_TARGET,
};

impl<T: Config> Pallet<T> {
    /// Calculate expected consumers counter for a `who` account, and if actual
    /// counter is not as expected, increment or decrement current counter
    pub fn fix_consumer_counter(who: T::AccountId) -> DispatchResult {
        let current_consumers = T::AccountInfoProvider::get_consumers(&who);
        let mut expected_consumers: u32 = 0;

        if Self::reserved_or_frozen_non_zero(&who) {
            expected_consumers += 1;
        }
        if Self::is_contract_account(&who) {
            expected_consumers += 1;
        }
        if Self::is_bonded(&who) {
            expected_consumers += 1;
        }
        if Self::has_next_session_keys_and_account_is_controller(&who) {
            expected_consumers += 1;
        }

        #[allow(clippy::comparison_chain)]
        if current_consumers < expected_consumers {
            log::debug!(
                target: LOG_TARGET,
                "Account {:?} has consumers underflow: current({}) < expected ({}), incrementing ",
                HexDisplay::from(&who.encode()), current_consumers, expected_consumers);
            Self::increment_consumers(&who)?;
        } else if current_consumers > expected_consumers {
            log::debug!(
                target: LOG_TARGET,
                "Account {:?} has consumers overflow: current({}) > expected ({}), decrementing ",
                HexDisplay::from(&who.encode()), current_consumers, expected_consumers);
            Self::decrement_consumers(&who);
        } else {
            log::trace!(
                target: LOG_TARGET,
                "Account {:?} neither has underflow nor overflow of consumers counter.",
                HexDisplay::from(&who.encode())
            );
        }

        Ok(())
    }

    fn reserved_or_frozen_non_zero(who: &T::AccountId) -> bool {
        !T::BalancesProvider::is_reserved_zero(who) || !T::BalancesProvider::is_frozen_zero(who)
    }

    fn is_bonded(who: &T::AccountId) -> bool {
        T::BondedStashProvider::get_controller(who).is_some()
    }

    fn is_contract_account(who: &T::AccountId) -> bool {
        T::ContractInfoProvider::is_contract_account(who)
    }

    fn has_next_session_keys_and_account_is_controller(who: &T::AccountId) -> bool {
        let has_next_session_keys = T::NextKeysSessionProvider::has_next_session_keys(who);
        let stash_equal_to_controller = match T::BondedStashProvider::get_controller(who) {
            Some(controller) => *who == controller,
            None => false,
        };
        if has_next_session_keys && stash_equal_to_controller {
            return true;
        }
        match T::BondedStashProvider::get_stash(who) {
            Some(stash) => {
                *who != stash && T::NextKeysSessionProvider::has_next_session_keys(&stash)
            }
            None => false,
        }
    }

    fn increment_consumers(who: &T::AccountId) -> Result<(), DispatchError> {
        frame_system::Pallet::<T>::inc_consumers_without_limit(who)?;
        Self::deposit_event(Event::ConsumersCounterIncremented { who: who.clone() });
        Ok(())
    }

    fn decrement_consumers(who: &T::AccountId) {
        // dec_consumers does not return any error when current counter is 0, hence we need to
        // handle such case manually
        if T::AccountInfoProvider::get_consumers(who) > 0 {
            frame_system::Pallet::<T>::dec_consumers(who);
            Self::deposit_event(Event::ConsumersCounterDecremented { who: who.clone() });
        }
    }
}
