#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]

extern crate core;

mod impls;
mod traits;

#[cfg(test)]
mod tests;

use frame_support::traits::{LockIdentifier, StorageVersion};

const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);
pub const LOG_TARGET: &str = "pallet-operations";
// harcoding as those consts are not public in substrate
pub const STAKING_ID: LockIdentifier = *b"staking ";
pub const VESTING_ID: LockIdentifier = *b"vesting ";

pub use pallet::*;

#[frame_support::pallet]
#[pallet_doc("../README.md")]
pub mod pallet {
    use frame_support::{pallet_prelude::*, weights::constants::WEIGHT_REF_TIME_PER_MILLIS};
    use frame_system::{ensure_signed, pallet_prelude::OriginFor};

    use crate::{
        traits::{AccountInfoProvider, BalancesProvider, NextKeysSessionProvider},
        STORAGE_VERSION,
    };

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Something that provides information about an account's consumers counter
        type AccountInfoProvider: AccountInfoProvider<AccountId = Self::AccountId, RefCount = u32>;
        type BalancesProvider: BalancesProvider<AccountId = Self::AccountId>;
        type NextKeysSessionProvider: NextKeysSessionProvider<AccountId = Self::AccountId>;
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// An account has fixed its consumers counter underflow
        ConsumersUnderflowFixed { who: T::AccountId },
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// An account can have an underflow of a `consumers` counter.
        /// Account categories that are impacted by this issue depends on a chain runtime,
        /// but specifically for AlephNode runtime are as follows:
        /// * `consumers`  == 0, `reserved`  > 0
        /// * `consumers`  == 1, `balances.Locks` contain an entry with `id`  == `vesting`
        /// * `consumers`  == 2, `balances.Locks` contain an entry with `id`  == `staking`
        /// * `consumers`  == 3, `balances.Locks` contain entries with `id`  == `staking`
        ///    and account id is in `session.nextKeys`
        ///
        ///	`fix_accounts_consumers_underflow` checks if the account falls into one of above
        /// categories, and increase its `consumers` counter.
        ///
        /// - `origin`: Must be `Signed`.
        /// - `who`: An account to be fixed
        ///
        #[pallet::call_index(0)]
        #[pallet::weight(
        Weight::from_parts(WEIGHT_REF_TIME_PER_MILLIS.saturating_mul(8), 0)
        )]
        pub fn fix_accounts_consumers_underflow(
            origin: OriginFor<T>,
            who: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            ensure_signed(origin)?;
            Self::fix_underflow_consumer_counter(who)?;
            Ok(().into())
        }
    }
}
