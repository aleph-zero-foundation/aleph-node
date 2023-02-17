//! This pallet manages changes in the committee responsible for producing blocks and establishing consensus.
//!
//! # Terminology
//! For definition of session, era, staking see pallet_session and pallet_staking.
//! - committee ([`EraValidators`]): Set of nodes that produce and finalize blocks in the session.
//! - validator: Node that can become a member of committee (or already is) via rotation.
//! - `EraValidators::reserved`: immutable validators, ie they cannot be removed from that list.
//! - `EraValidators::non_reserved`: validators that can be banned out from that list.
//!
//! # Elections process
//! There are two options for choosing validators during election process governed by ([`Openness`]) storage value:
//! - `Permissionless`: choose all validators that bonded enough amount and are not banned.
//! - `Permissioned`: choose `EraValidators::reserved` and all `EraValidators::non_reserved` that are not banned.
//!
//! # Ban logic
//! In case of insufficient validator's uptime, we need to remove such validators from
//! the committee, so that the network is as healthy as possible. This is achieved by calculating
//! number of _underperformance_ sessions, which means that number of blocks produced by the
//! validator is less than some predefined threshold.
//! In other words, if a validator:
//! * performance in a session is less or equal to a configurable threshold
//! `BanConfig::minimal_expected_performance` (from 0 to 100%), and,
//! * it happened at least `BanConfig::underperformed_session_count_threshold` times,
//! then the validator is considered an underperformer and hence removed (ie _banned out_) from the
//! committee.
//!
//! ## Thresholds
//! There are two ban thresholds described above, see [`BanConfig`].
//!
//! ### Next era vs current era
//! Current and next era have distinct thresholds values, as we calculate bans during elections.
//! They follow the same logic as next era committee seats: at the time of planning the first
//! session of next the era, next values become current ones.

#![cfg_attr(not(feature = "std"), no_std)]

mod impls;
mod migrations;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
mod traits;

use codec::{Decode, Encode};
use frame_support::{log::info, traits::StorageVersion};
pub use impls::{compute_validator_scaled_total_rewards, LENIENT_THRESHOLD};
pub use pallet::*;
use pallets_support::StorageMigration;
pub use primitives::EraValidators;
use scale_info::TypeInfo;
use sp_std::{
    collections::{btree_map::BTreeMap, btree_set::BTreeSet},
    prelude::*,
};

pub type TotalReward = u32;

const STORAGE_VERSION: StorageVersion = StorageVersion::new(3);

#[derive(Decode, Encode, TypeInfo)]
pub struct ValidatorTotalRewards<T>(pub BTreeMap<T, TotalReward>);

#[frame_support::pallet]
pub mod pallet {
    use frame_election_provider_support::{
        BoundedSupportsOf, ElectionDataProvider, ElectionProvider, ElectionProviderBase, Support,
        Supports,
    };
    use frame_support::{log, pallet_prelude::*, traits::Get};
    use frame_system::{
        ensure_root,
        pallet_prelude::{BlockNumberFor, OriginFor},
    };
    use pallet_session::SessionManager;
    use primitives::{
        BanConfig as BanConfigStruct, BanInfo, BanReason, BlockCount, CommitteeSeats,
        ElectionOpenness, EraIndex, SessionCount,
    };
    use sp_runtime::Perbill;

    use super::*;
    use crate::traits::{
        EraInfoProvider, SessionInfoProvider, ValidatorExtractor, ValidatorRewardsHandler,
    };

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Something that provides information about ongoing eras.
        type EraInfoProvider: EraInfoProvider<AccountId = Self::AccountId>;
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Something that provides data for elections.
        type DataProvider: ElectionDataProvider<
            AccountId = Self::AccountId,
            BlockNumber = Self::BlockNumber,
        >;
        /// Nr of blocks in the session.
        #[pallet::constant]
        type SessionPeriod: Get<u32>;
        /// Handler for managing new session.
        type SessionManager: SessionManager<<Self as frame_system::Config>::AccountId>;
        /// Something that provides information about sessions.
        type SessionInfoProvider: SessionInfoProvider<Self>;
        /// Something that handles addition of rewards for validators.
        type ValidatorRewardsHandler: ValidatorRewardsHandler<Self>;
        /// Something that removes validators from candidates in elections
        type ValidatorExtractor: ValidatorExtractor<AccountId = Self::AccountId>;

        /// Maximum acceptable ban reason length.
        #[pallet::constant]
        type MaximumBanReasonLength: Get<u32>;

        /// The maximum number of winners that can be elected by this `ElectionProvider`
        /// implementation.
        ///
        /// Note: This must always be greater or equal to `T::DataProvider::desired_targets()`.
        #[pallet::constant]
        type MaxWinners: Get<u32>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Committee for the next era has changed
        ChangeValidators(Vec<T::AccountId>, Vec<T::AccountId>, CommitteeSeats),

        /// Ban thresholds for the next era has changed
        SetBanConfig(BanConfigStruct),

        /// Validators have been banned from the committee
        BanValidators(Vec<(T::AccountId, BanInfo)>),
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_runtime_upgrade() -> frame_support::weights::Weight {
            let on_chain = <Pallet<T> as GetStorageVersion>::on_chain_storage_version();
            T::DbWeight::get().reads(1)
                + match on_chain {
                    _ if on_chain == STORAGE_VERSION => Weight::zero(),
                    _ if on_chain == StorageVersion::new(0) => {
                        migrations::v0_to_v1::Migration::<T, Self>::migrate()
                            + migrations::v1_to_v2::Migration::<T, Self>::migrate()
                            + migrations::v2_to_v3::Migration::<T, Self>::migrate()
                    }
                    _ if on_chain == StorageVersion::new(1) => {
                        migrations::v1_to_v2::Migration::<T, Self>::migrate()
                            + migrations::v2_to_v3::Migration::<T, Self>::migrate()
                    }
                    _ if on_chain == StorageVersion::new(2) => {
                        migrations::v2_to_v3::Migration::<T, Self>::migrate()
                    }
                    _ => {
                        log::warn!(
                            target: "pallet_elections",
                            "On chain storage version of pallet elections is {:?} but it should not be bigger than 2",
                            on_chain
                        );
                        Weight::zero()
                    }
                }
        }
    }
    /// Desirable size of a committee, see [`CommitteeSeats`].
    #[pallet::storage]
    pub type CommitteeSize<T> = StorageValue<_, CommitteeSeats, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultNextEraCommitteeSize<T: Config>() -> CommitteeSeats {
        CommitteeSize::<T>::get()
    }

    /// Desired size of a committee in effect from a new era.
    #[pallet::storage]
    pub type NextEraCommitteeSize<T> =
        StorageValue<_, CommitteeSeats, ValueQuery, DefaultNextEraCommitteeSize<T>>;

    /// Next era's list of reserved validators.
    #[pallet::storage]
    pub type NextEraReservedValidators<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

    /// Current era's list of reserved validators.
    #[pallet::storage]
    pub type CurrentEraValidators<T: Config> =
        StorageValue<_, EraValidators<T::AccountId>, ValueQuery>;

    /// Next era's list of non reserved validators.
    #[pallet::storage]
    pub type NextEraNonReservedValidators<T: Config> =
        StorageValue<_, Vec<T::AccountId>, ValueQuery>;

    /// A lookup how many blocks a validator produced.
    #[pallet::storage]
    pub type SessionValidatorBlockCount<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, BlockCount, ValueQuery>;

    /// Total possible reward per validator for the current era.
    #[pallet::storage]
    pub type ValidatorEraTotalReward<T: Config> =
        StorageValue<_, ValidatorTotalRewards<T::AccountId>, OptionQuery>;

    /// Default value for ban config, see [`BanConfig`]
    #[pallet::type_value]
    pub fn DefaultBanConfig<T: Config>() -> BanConfigStruct {
        BanConfigStruct::default()
    }

    /// Current era config for ban functionality, see [`BanConfig`]
    #[pallet::storage]
    pub type BanConfig<T> = StorageValue<_, BanConfigStruct, ValueQuery, DefaultBanConfig<T>>;

    /// A lookup for a number of underperformance sessions for a given validator
    #[pallet::storage]
    pub type UnderperformedValidatorSessionCount<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, SessionCount, ValueQuery>;

    /// Validators to be removed from non reserved list in the next era
    #[pallet::storage]
    pub type Banned<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, BanInfo>;

    /// Default value for elections openness.
    #[pallet::type_value]
    pub fn DefaultOpenness<T: Config>() -> ElectionOpenness {
        ElectionOpenness::Permissioned
    }

    /// Openness of the elections, whether we allow all candidates that bonded enough tokens or
    /// the validators list is managed by sudo
    #[pallet::storage]
    pub type Openness<T> = StorageValue<_, ElectionOpenness, ValueQuery, DefaultOpenness<T>>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight((T::BlockWeights::get().max_block, DispatchClass::Operational))]
        pub fn change_validators(
            origin: OriginFor<T>,
            reserved_validators: Option<Vec<T::AccountId>>,
            non_reserved_validators: Option<Vec<T::AccountId>>,
            committee_size: Option<CommitteeSeats>,
        ) -> DispatchResult {
            ensure_root(origin)?;
            let committee_size = committee_size.unwrap_or_else(NextEraCommitteeSize::<T>::get);
            let reserved_validators =
                reserved_validators.unwrap_or_else(NextEraReservedValidators::<T>::get);
            let non_reserved_validators =
                non_reserved_validators.unwrap_or_else(NextEraNonReservedValidators::<T>::get);

            Self::ensure_validators_are_ok(
                reserved_validators.clone(),
                non_reserved_validators.clone(),
                committee_size,
            )?;

            NextEraNonReservedValidators::<T>::put(non_reserved_validators.clone());
            NextEraReservedValidators::<T>::put(reserved_validators.clone());
            NextEraCommitteeSize::<T>::put(committee_size);

            Self::deposit_event(Event::ChangeValidators(
                reserved_validators,
                non_reserved_validators,
                committee_size,
            ));

            Ok(())
        }

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

        /// Schedule a non-reserved node to be banned out from the committee at the end of the era
        #[pallet::call_index(3)]
        #[pallet::weight((T::BlockWeights::get().max_block, DispatchClass::Operational))]
        pub fn cancel_ban(origin: OriginFor<T>, banned: T::AccountId) -> DispatchResult {
            ensure_root(origin)?;
            Banned::<T>::remove(banned);

            Ok(())
        }

        /// Set openness of the elections
        #[pallet::call_index(4)]
        #[pallet::weight((T::BlockWeights::get().max_block, DispatchClass::Operational))]
        pub fn set_elections_openness(
            origin: OriginFor<T>,
            openness: ElectionOpenness,
        ) -> DispatchResult {
            ensure_root(origin)?;

            Openness::<T>::set(openness);

            Ok(())
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub non_reserved_validators: Vec<T::AccountId>,
        pub reserved_validators: Vec<T::AccountId>,
        pub committee_seats: CommitteeSeats,
        pub committee_ban_config: BanConfigStruct,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                non_reserved_validators: Vec::new(),
                reserved_validators: Vec::new(),
                committee_seats: Default::default(),
                committee_ban_config: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            <CommitteeSize<T>>::put(self.committee_seats);
            <NextEraCommitteeSize<T>>::put(self.committee_seats);
            <NextEraNonReservedValidators<T>>::put(&self.non_reserved_validators);
            <NextEraReservedValidators<T>>::put(&self.reserved_validators);
            <CurrentEraValidators<T>>::put(&EraValidators {
                reserved: self.reserved_validators.clone(),
                non_reserved: self.non_reserved_validators.clone(),
            });
            <BanConfig<T>>::put(&self.committee_ban_config.clone());
        }
    }

    impl<T: Config> Pallet<T> {
        fn ensure_validators_are_ok(
            reserved_validators: Vec<T::AccountId>,
            non_reserved_validators: Vec<T::AccountId>,
            committee_size: CommitteeSeats,
        ) -> DispatchResult {
            let CommitteeSeats {
                reserved_seats: reserved,
                non_reserved_seats: non_reserved,
            } = committee_size;
            let reserved_len = reserved_validators.len() as u32;
            let non_reserved_len = non_reserved_validators.len() as u32;
            let validators_size = reserved_len + non_reserved_len;

            let committee_size_all = reserved + non_reserved;

            ensure!(
                committee_size_all <= validators_size,
                Error::<T>::NotEnoughValidators
            );

            ensure!(
                reserved <= reserved_len,
                Error::<T>::NotEnoughReservedValidators,
            );

            ensure!(
                non_reserved <= non_reserved_len,
                Error::<T>::NotEnoughReservedValidators,
            );

            let member_set: BTreeSet<_> = reserved_validators
                .into_iter()
                .chain(non_reserved_validators.into_iter())
                .collect();

            ensure!(
                member_set.len() as u32 == validators_size,
                Error::<T>::NonUniqueListOfValidators
            );

            Ok(())
        }

        fn emit_fresh_bans_event() {
            let active_era = <T as Config>::EraInfoProvider::active_era().unwrap_or(1);
            let fresh_bans = Banned::<T>::iter()
                .filter(|(_acc, info)| info.start == active_era + 1)
                .collect::<Vec<_>>();
            if !fresh_bans.is_empty() {
                info!(target: "pallet_elections", "Fresh bans in era {}: {:?}",active_era, fresh_bans);
                Self::deposit_event(Event::BanValidators(fresh_bans));
            }
        }
    }

    #[derive(Debug)]
    pub enum ElectionError {
        DataProvider(&'static str),

        /// Winner number is greater than
        /// [`Config::MaxWinners`]
        TooManyWinners,
    }

    #[pallet::error]
    pub enum Error<T> {
        NotEnoughValidators,
        NotEnoughReservedValidators,
        NotEnoughNonReservedValidators,
        NonUniqueListOfValidators,

        /// Raised in any scenario [`BanConfig`] is invalid
        /// * `performance_ratio_threshold` must be a number in range [0; 100]
        /// * `underperformed_session_count_threshold` must be a positive number,
        /// * `clean_session_counter_delay` must be a positive number.
        InvalidBanConfig,

        /// Ban reason is too big, ie given vector of bytes is greater than
        /// [`Config::MaximumBanReasonLength`]
        BanReasonTooBig,
    }

    impl<T: Config> ElectionProviderBase for Pallet<T> {
        type AccountId = T::AccountId;
        type BlockNumber = T::BlockNumber;
        type Error = ElectionError;
        type DataProvider = T::DataProvider;
        type MaxWinners = T::MaxWinners;
    }

    impl<T: Config> ElectionProvider for Pallet<T> {
        fn ongoing() -> bool {
            false
        }

        /// We calculate the supports for each validator. The external validators are chosen as:
        /// 1) "`NextEraNonReservedValidators` that are staking and are not banned" in case of Permissioned ElectionOpenness
        /// 2) "All staking and not banned validators" in case of Permissionless ElectionOpenness
        fn elect() -> Result<BoundedSupportsOf<Self>, Self::Error> {
            Self::emit_fresh_bans_event();
            let active_era = <T as Config>::EraInfoProvider::active_era().unwrap_or(0);
            let ban_period = BanConfig::<T>::get().ban_period;

            let staking_validators = Self::DataProvider::electable_targets(None)
                .map_err(Self::Error::DataProvider)?
                .into_iter()
                .collect::<BTreeSet<_>>();
            let staking_reserved_validators = NextEraReservedValidators::<T>::get()
                .into_iter()
                .filter(|v| staking_validators.contains(v))
                .collect::<BTreeSet<_>>();
            let banned_validators = Banned::<T>::iter()
                .filter(|(_, info)| !Self::ban_expired(info.start, ban_period, active_era + 1))
                .map(|(v, _)| v)
                .collect::<BTreeSet<_>>();
            let old_non_reserved_validators = NextEraNonReservedValidators::<T>::get().into_iter();

            let eligible_non_reserved = staking_validators
                .into_iter()
                .filter(|v| {
                    !banned_validators.contains(v) && !staking_reserved_validators.contains(v)
                })
                .collect::<BTreeSet<_>>();

            let new_non_reserved_validators: Vec<_> = match Openness::<T>::get() {
                ElectionOpenness::Permissioned => old_non_reserved_validators
                    .filter(|v| eligible_non_reserved.contains(v))
                    .collect(),
                ElectionOpenness::Permissionless => eligible_non_reserved.into_iter().collect(),
            };
            // We store new list here to ensure that validators that end up in the result of the elect
            // method are a disjoint union of NextEraReservedValidators and NextEraNonReservedValidators.
            // This condition is important since results of elect ends up in pallet staking while the above lists
            // are used in our session manager, so we have to ensure consistency between them.
            NextEraNonReservedValidators::<T>::put(new_non_reserved_validators.clone());

            let eligible_validators = staking_reserved_validators
                .into_iter()
                .chain(new_non_reserved_validators.into_iter());
            let mut supports = eligible_validators
                .into_iter()
                .map(|id| {
                    (
                        id,
                        // Under normal circumstances support will never be `0` since 'self-vote'
                        // is counted in.
                        Support {
                            total: 0,
                            voters: Vec::new(),
                        },
                    )
                })
                .collect::<BTreeMap<_, _>>();

            let voters =
                Self::DataProvider::electing_voters(None).map_err(Self::Error::DataProvider)?;
            for (voter, vote, targets) in voters {
                // The parameter `Staking::MAX_NOMINATIONS` is set to 1 which guarantees that
                // `len(targets) == 1`.
                let member = &targets[0];
                if let Some(support) = supports.get_mut(member) {
                    support.total += vote as u128;
                    support.voters.push((voter, vote as u128));
                }
            }

            supports
                .into_iter()
                .collect::<Supports<_>>()
                .try_into()
                .map_err(|_| Self::Error::TooManyWinners)
        }
    }
}
