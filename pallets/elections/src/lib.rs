#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]

mod impls;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
mod traits;

use frame_support::traits::StorageVersion;
pub use pallet::*;
use parity_scale_codec::{Decode, Encode};
pub use primitives::EraValidators;
use scale_info::TypeInfo;
use sp_std::{
    collections::{btree_map::BTreeMap, btree_set::BTreeSet},
    prelude::*,
};

pub type TotalReward = u32;

const STORAGE_VERSION: StorageVersion = StorageVersion::new(5);

#[derive(Decode, Encode, TypeInfo)]
pub struct ValidatorTotalRewards<T>(pub BTreeMap<T, TotalReward>);

#[frame_support::pallet]
#[pallet_doc("../README.md")]
pub mod pallet {
    use frame_election_provider_support::{
        BoundedSupportsOf, ElectionDataProvider, ElectionProvider, ElectionProviderBase, Support,
        Supports,
    };
    use frame_support::{pallet_prelude::*, traits::Get};
    use frame_system::{
        ensure_root,
        pallet_prelude::{BlockNumberFor, OriginFor},
    };
    use primitives::{BannedValidators, CommitteeSeats, ElectionOpenness};

    use super::*;
    use crate::traits::ValidatorProvider;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Something that provides data for elections.
        type DataProvider: ElectionDataProvider<
            AccountId = Self::AccountId,
            BlockNumber = BlockNumberFor<Self>,
        >;
        type ValidatorProvider: ValidatorProvider<AccountId = Self::AccountId>;
        /// The maximum number of winners that can be elected by this `ElectionProvider`
        /// implementation.
        ///
        /// Note: This must always be greater or equal to `T::DataProvider::desired_targets()`.
        #[pallet::constant]
        type MaxWinners: Get<u32>;
        type BannedValidators: BannedValidators<AccountId = Self::AccountId>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Committee for the next era has changed
        ChangeValidators(Vec<T::AccountId>, Vec<T::AccountId>, CommitteeSeats),
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

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

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        #[cfg(feature = "try-runtime")]
        fn try_state(_n: BlockNumberFor<T>) -> Result<(), DispatchError> {
            let current_validators = CurrentEraValidators::<T>::get();
            Self::ensure_validators_are_ok(
                current_validators.reserved,
                current_validators.non_reserved,
                CommitteeSize::<T>::get(),
            )?;

            Self::ensure_validators_are_ok(
                NextEraReservedValidators::<T>::get(),
                NextEraNonReservedValidators::<T>::get(),
                NextEraCommitteeSize::<T>::get(),
            )?;

            Ok(())
        }
    }

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        pub non_reserved_validators: Vec<T::AccountId>,
        pub reserved_validators: Vec<T::AccountId>,
        pub committee_seats: CommitteeSeats,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            <CommitteeSize<T>>::put(self.committee_seats);
            <NextEraCommitteeSize<T>>::put(self.committee_seats);
            <NextEraNonReservedValidators<T>>::put(&self.non_reserved_validators);
            <NextEraReservedValidators<T>>::put(&self.reserved_validators);
            <CurrentEraValidators<T>>::put(&EraValidators {
                reserved: self.reserved_validators.clone(),
                non_reserved: self.non_reserved_validators.clone(),
            });
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
                non_reserved_finality_seats: non_reserved_finality,
            } = committee_size;
            let reserved_len = reserved_validators.len() as u32;
            let non_reserved_len = non_reserved_validators.len() as u32;
            let validators_size = reserved_len + non_reserved_len;

            let committee_size_all = reserved + non_reserved;

            ensure!(
                non_reserved_finality <= non_reserved,
                Error::<T>::NonReservedFinalitySeatsLargerThanNonReservedSeats
            );
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
        NonReservedFinalitySeatsLargerThanNonReservedSeats,
    }

    impl<T: Config> ElectionProviderBase for Pallet<T> {
        type AccountId = T::AccountId;
        type BlockNumber = BlockNumberFor<T>;
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
            let staking_validators = Self::DataProvider::electable_targets(None)
                .map_err(Self::Error::DataProvider)?
                .into_iter()
                .collect::<BTreeSet<_>>();
            let staking_reserved_validators = NextEraReservedValidators::<T>::get()
                .into_iter()
                .filter(|v| staking_validators.contains(v))
                .collect::<BTreeSet<_>>();
            let banned_validators = T::BannedValidators::banned()
                .into_iter()
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
