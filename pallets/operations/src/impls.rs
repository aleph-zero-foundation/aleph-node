#![allow(clippy::nonminimal_bool)]

use frame_support::{dispatch::DispatchResult, traits::LockIdentifier, WeakBoundedVec};
use pallet_balances::BalanceLock;
use parity_scale_codec::Encode;
use sp_core::hexdisplay::HexDisplay;
use sp_runtime::DispatchError;

use crate::{
    pallet::{Config, Event, Pallet},
    traits::{AccountInfoProvider, BalancesProvider, BondedStashProvider, NextKeysSessionProvider},
    LOG_TARGET, STAKING_ID, VESTING_ID,
};

impl<T: Config> Pallet<T> {
    /// Checks if account has an underflow of `consumers` counter. In such case, it increments
    /// it by one.
    pub fn fix_underflow_consumer_counter(who: T::AccountId) -> DispatchResult {
        let current_consumers = T::AccountInfoProvider::get_consumers(&who);
        let mut expected_consumers: u32 = 0;

        if Self::reserved_or_frozen_non_zero(&who) {
            expected_consumers += 1;
        }
        let has_vesting_lock = Self::has_vesting_lock(&who);
        let has_staking_lock = Self::has_staking_lock(&who);
        if has_staking_lock || has_vesting_lock {
            expected_consumers += 1;
            if has_staking_lock {
                expected_consumers += 1;
            }
        }
        if Self::has_next_session_keys_and_account_is_controller(&who) {
            expected_consumers += 1;
        }

        if current_consumers < expected_consumers {
            log::debug!(
                target: LOG_TARGET,
                "Account {:?} has current consumers {} less than expected consumers {:?}, incrementing ",
                HexDisplay::from(&who.encode()), current_consumers, expected_consumers);
            Self::increment_consumers(who)?;
        } else {
            log::debug!(
                target: LOG_TARGET,
                "Account {:?} does not have consumers underflow, not incrementing",
                HexDisplay::from(&who.encode())
            );
        }

        Ok(())
    }

    fn reserved_or_frozen_non_zero(who: &T::AccountId) -> bool {
        !T::BalancesProvider::is_reserved_zero(who) || !T::BalancesProvider::is_frozen_zero(who)
    }

    fn has_vesting_lock(who: &T::AccountId) -> bool {
        let locks = T::BalancesProvider::locks(who);
        Self::has_lock(&locks, VESTING_ID)
    }

    fn has_staking_lock(who: &T::AccountId) -> bool {
        let locks = T::BalancesProvider::locks(who);
        Self::has_lock(&locks, STAKING_ID)
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

    fn has_lock<U, V>(locks: &WeakBoundedVec<BalanceLock<U>, V>, id: LockIdentifier) -> bool {
        locks.iter().any(|x| x.id == id)
    }

    fn increment_consumers(who: T::AccountId) -> Result<(), DispatchError> {
        frame_system::Pallet::<T>::inc_consumers_without_limit(&who)?;
        Self::deposit_event(Event::ConsumersUnderflowFixed { who });
        Ok(())
    }
}
