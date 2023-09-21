#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]

extern crate core;

mod impls;
mod manager;
mod traits;

use frame_support::{pallet_prelude::Get, traits::StorageVersion};
pub use manager::SessionAndEraManager;
pub use pallet::*;
use parity_scale_codec::{Decode, Encode};
use primitives::{BanConfig as BanConfigStruct, BanInfo, SessionValidators, LENIENT_THRESHOLD};
use scale_info::TypeInfo;
use sp_runtime::Perquintill;
use sp_std::{collections::btree_map::BTreeMap, default::Default};
pub use traits::*;

pub type TotalReward = u32;
#[derive(Decode, Encode, TypeInfo, PartialEq, Eq)]
pub struct ValidatorTotalRewards<T>(pub BTreeMap<T, TotalReward>);

#[derive(Decode, Encode, TypeInfo)]
struct CurrentAndNextSessionValidators<T> {
    pub next: SessionValidators<T>,
    pub current: SessionValidators<T>,
}

impl<T> Default for CurrentAndNextSessionValidators<T> {
    fn default() -> Self {
        Self {
            next: Default::default(),
            current: Default::default(),
        }
    }
}

pub struct DefaultLenientThreshold;

impl Get<Perquintill> for DefaultLenientThreshold {
    fn get() -> Perquintill {
        LENIENT_THRESHOLD
    }
}

const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);
pub(crate) const LOG_TARGET: &str = "pallet-committee-management";

#[frame_support::pallet]
#[pallet_doc("../README.md")]
pub mod pallet {
    use frame_support::{
        dispatch::DispatchResult, ensure, pallet_prelude::*, BoundedVec, Twox64Concat,
    };
    use frame_system::{ensure_root, pallet_prelude::OriginFor};
    use primitives::{
        BanHandler, BanReason, BlockCount, FinalityCommitteeManager, SessionCount,
        SessionValidators, ValidatorProvider,
    };
    use sp_runtime::{Perbill, Perquintill};
    use sp_staking::EraIndex;
    use sp_std::vec::Vec;

    use crate::{
        traits::{EraInfoProvider, ValidatorRewardsHandler},
        BanConfigStruct, BanInfo, CurrentAndNextSessionValidators, DefaultLenientThreshold,
        ValidatorExtractor, ValidatorTotalRewards, STORAGE_VERSION,
    };

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Something that handles bans
        type BanHandler: BanHandler<AccountId = Self::AccountId>;
        /// Something that provides information about era.
        type EraInfoProvider: EraInfoProvider<AccountId = Self::AccountId>;
        /// Something that provides information about validator.
        type ValidatorProvider: ValidatorProvider<AccountId = Self::AccountId>;
        /// Something that handles addition of rewards for validators.
        type ValidatorRewardsHandler: ValidatorRewardsHandler<AccountId = Self::AccountId>;
        /// Something that handles removal of the validators
        type ValidatorExtractor: ValidatorExtractor<AccountId = Self::AccountId>;
        type FinalityCommitteeManager: FinalityCommitteeManager<Self::AccountId>;
        /// Nr of blocks in the session.
        #[pallet::constant]
        type SessionPeriod: Get<u32>;
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    pub type LenientThreshold<T: Config> =
        StorageValue<_, Perquintill, ValueQuery, DefaultLenientThreshold>;

    /// A lookup how many blocks a validator produced.
    #[pallet::storage]
    pub type SessionValidatorBlockCount<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, BlockCount, ValueQuery>;

    /// Total possible reward per validator for the current era.
    #[pallet::storage]
    pub type ValidatorEraTotalReward<T: Config> =
        StorageValue<_, ValidatorTotalRewards<T::AccountId>, OptionQuery>;

    /// Current era config for ban functionality, see [`BanConfig`]
    #[pallet::storage]
    pub type BanConfig<T> = StorageValue<_, BanConfigStruct, ValueQuery>;

    /// A lookup for a number of underperformance sessions for a given validator
    #[pallet::storage]
    pub type UnderperformedValidatorSessionCount<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, SessionCount, ValueQuery>;

    /// Validators to be removed from non reserved list in the next era
    #[pallet::storage]
    pub type Banned<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, BanInfo>;

    /// SessionValidators in the current session.
    #[pallet::storage]
    pub(crate) type CurrentAndNextSessionValidatorsStorage<T: Config> =
        StorageValue<_, CurrentAndNextSessionValidators<T::AccountId>, ValueQuery>;

    #[pallet::error]
    pub enum Error<T> {
        /// Raised in any scenario [`BanConfig`] is invalid
        /// * `performance_ratio_threshold` must be a number in range [0; 100]
        /// * `underperformed_session_count_threshold` must be a positive number,
        /// * `clean_session_counter_delay` must be a positive number.
        InvalidBanConfig,

        /// Ban reason is too big, ie given vector of bytes is greater than
        /// [`primitives::DEFAULT_BAN_REASON_LENGTH`]
        BanReasonTooBig,

        /// Lenient threshold not in [0-100] range
        InvalidLenientThreshold,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Ban thresholds for the next era has changed
        SetBanConfig(BanConfigStruct),

        /// Validators have been banned from the committee
        BanValidators(Vec<(T::AccountId, BanInfo)>),
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Sets ban config, it has an immediate effect
        #[pallet::call_index(1)]
        #[pallet::weight((T::BlockWeights::get().max_block, DispatchClass::Operational))]
        pub fn set_ban_config(
            origin: OriginFor<T>,
            minimal_expected_performance: Option<u8>,
            underperformed_session_count_threshold: Option<u32>,
            clean_session_counter_delay: Option<u32>,
            ban_period: Option<EraIndex>,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let mut current_committee_ban_config = BanConfig::<T>::get();

            if let Some(minimal_expected_performance) = minimal_expected_performance {
                ensure!(
                    minimal_expected_performance <= 100,
                    Error::<T>::InvalidBanConfig
                );
                current_committee_ban_config.minimal_expected_performance =
                    Perbill::from_percent(minimal_expected_performance as u32);
            }
            if let Some(underperformed_session_count_threshold) =
                underperformed_session_count_threshold
            {
                ensure!(
                    underperformed_session_count_threshold > 0,
                    Error::<T>::InvalidBanConfig
                );
                current_committee_ban_config.underperformed_session_count_threshold =
                    underperformed_session_count_threshold;
            }
            if let Some(clean_session_counter_delay) = clean_session_counter_delay {
                ensure!(
                    clean_session_counter_delay > 0,
                    Error::<T>::InvalidBanConfig
                );
                current_committee_ban_config.clean_session_counter_delay =
                    clean_session_counter_delay;
            }
            if let Some(ban_period) = ban_period {
                ensure!(ban_period > 0, Error::<T>::InvalidBanConfig);
                current_committee_ban_config.ban_period = ban_period;
            }

            BanConfig::<T>::put(current_committee_ban_config.clone());
            Self::deposit_event(Event::SetBanConfig(current_committee_ban_config));

            Ok(())
        }

        /// Schedule a non-reserved node to be banned out from the committee at the end of the era
        #[pallet::call_index(2)]
        #[pallet::weight((T::BlockWeights::get().max_block, DispatchClass::Operational))]
        pub fn ban_from_committee(
            origin: OriginFor<T>,
            banned: T::AccountId,
            ban_reason: Vec<u8>,
        ) -> DispatchResult {
            ensure_root(origin)?;
            let bounded_description: BoundedVec<_, _> = ban_reason
                .try_into()
                .map_err(|_| Error::<T>::BanReasonTooBig)?;

            let reason = BanReason::OtherReason(bounded_description);
            Self::ban_validator(&banned, reason);

            Ok(())
        }

        /// Cancel the ban of the node
        #[pallet::call_index(3)]
        #[pallet::weight((T::BlockWeights::get().max_block, DispatchClass::Operational))]
        pub fn cancel_ban(origin: OriginFor<T>, banned: T::AccountId) -> DispatchResult {
            ensure_root(origin)?;
            Banned::<T>::remove(banned);

            Ok(())
        }

        /// Set lenient threshold
        #[pallet::call_index(4)]
        #[pallet::weight((T::BlockWeights::get().max_block, DispatchClass::Operational))]
        pub fn set_lenient_threshold(
            origin: OriginFor<T>,
            threshold_percent: u8,
        ) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(
                threshold_percent <= 100,
                Error::<T>::InvalidLenientThreshold
            );

            LenientThreshold::<T>::put(Perquintill::from_percent(threshold_percent as u64));

            Ok(())
        }
    }

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        pub committee_ban_config: BanConfigStruct,
        pub session_validators: SessionValidators<T::AccountId>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            <BanConfig<T>>::put(self.committee_ban_config.clone());
            <CurrentAndNextSessionValidatorsStorage<T>>::put(CurrentAndNextSessionValidators {
                current: self.session_validators.clone(),
                next: self.session_validators.clone(),
            })
        }
    }
}
