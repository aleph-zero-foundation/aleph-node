//! This pallet manages changes in the committee responsible for producing blocks and establishing consensus.
//! Currently, it's PoA where the validators are set by the root account. In the future, a new
//! version for DPoS elections will replace the current one.
//!
//! ### Terminology
//! For definition of session, era, staking see pallet_session and pallet_staking.
//! - Committee: Set of nodes that produce and finalize blocks in the session.
//! - Validator: Node that can become a member of committee (or already is) via rotation.
//! - ReservedValidators: Validators that are chosen to be in committee every single session.

#![cfg_attr(not(feature = "std"), no_std)]

mod impls;
mod migrations;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
mod traits;

use codec::{Decode, Encode};
use frame_support::traits::StorageVersion;
pub use impls::{compute_validator_scaled_total_rewards, LENIENT_THRESHOLD};
pub use pallet::*;
use pallets_support::StorageMigration;
pub use primitives::EraValidators;
use scale_info::TypeInfo;
use sp_std::{
    collections::{btree_map::BTreeMap, btree_set::BTreeSet},
    prelude::*,
};

const STORAGE_VERSION: StorageVersion = StorageVersion::new(3);

pub type BlockCount = u32;
pub type TotalReward = u32;

#[derive(Decode, Encode, TypeInfo)]
pub struct ValidatorTotalRewards<T>(pub BTreeMap<T, TotalReward>);

#[frame_support::pallet]
pub mod pallet {
    use frame_election_provider_support::{
        ElectionDataProvider, ElectionProvider, Support, Supports,
    };
    use frame_support::{log, pallet_prelude::*, traits::Get};
    use frame_system::{
        ensure_root,
        pallet_prelude::{BlockNumberFor, OriginFor},
    };
    use pallet_session::SessionManager;
    use primitives::CommitteeSeats;

    use super::*;
    use crate::traits::{EraInfoProvider, SessionInfoProvider, ValidatorRewardsHandler};

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Something that provides information about ongoing eras.
        type EraInfoProvider: EraInfoProvider<AccountId = Self::AccountId>;
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
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
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        ChangeValidators(Vec<T::AccountId>, Vec<T::AccountId>, CommitteeSeats),
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
                    _ if on_chain == STORAGE_VERSION => 0,
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
                        0
                    }
                }
        }
    }
    /// Desirable size of a committee.
    ///
    /// When new session is planned, first reserved validators are
    /// added to the committee. Then remaining slots are filled from total validators list excluding
    /// reserved validators
    #[pallet::storage]
    pub type CommitteeSize<T> = StorageValue<_, CommitteeSeats, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultNextEraCommitteeSize<T: Config>() -> CommitteeSeats {
        CommitteeSize::<T>::get()
    }

    /// Desired size of a committee in effect from a new era.
    ///
    /// can be changed via `change_validators` call that requires sudo.
    #[pallet::storage]
    pub type NextEraCommitteeSize<T> =
        StorageValue<_, CommitteeSeats, ValueQuery, DefaultNextEraCommitteeSize<T>>;

    /// List of reserved validators in force from a new era.
    ///
    /// Can be changed via `change_validators` call that requires sudo.
    #[pallet::storage]
    pub type NextEraReservedValidators<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

    /// Current era's list of reserved validators.
    ///
    /// This is populated from `NextEraReservedValidators`
    /// at the time of planning the first session of the era.
    /// This is a tuple of vectors representing `(reserved, non_reserved)` validators.
    #[pallet::storage]
    pub type CurrentEraValidators<T: Config> =
        StorageValue<_, EraValidators<T::AccountId>, ValueQuery>;

    /// List of possible validators that are not reserved.
    #[pallet::storage]
    pub type NextEraNonReservedValidators<T: Config> =
        StorageValue<_, Vec<T::AccountId>, ValueQuery>;

    /// Count per validator, how many blocks did the validator produced.
    #[pallet::storage]
    pub type SessionValidatorBlockCount<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, BlockCount, ValueQuery>;

    /// Total possible reward per validator for the current era.
    ///
    /// Scaled to fit in the u32.
    #[pallet::storage]
    pub type ValidatorEraTotalReward<T: Config> =
        StorageValue<_, ValidatorTotalRewards<T::AccountId>, OptionQuery>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
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
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub non_reserved_validators: Vec<T::AccountId>,
        pub reserved_validators: Vec<T::AccountId>,
        pub committee_seats: CommitteeSeats,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                non_reserved_validators: Vec::new(),
                reserved_validators: Vec::new(),
                committee_seats: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            <CommitteeSize<T>>::put(&self.committee_seats);
            <NextEraCommitteeSize<T>>::put(&self.committee_seats);
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
    }

    #[derive(Debug)]
    pub enum ElectionError {
        DataProvider(&'static str),
    }

    #[pallet::error]
    pub enum Error<T> {
        NotEnoughValidators,
        NotEnoughReservedValidators,
        NotEnoughNonReservedValidators,
        NonUniqueListOfValidators,
    }

    impl<T: Config> ElectionProvider for Pallet<T> {
        type AccountId = T::AccountId;
        type BlockNumber = T::BlockNumber;
        type Error = ElectionError;
        type DataProvider = T::DataProvider;

        /// The elections are PoA so only the nodes listed in the Validators will be elected as
        /// validators.
        ///
        /// We calculate the supports for them for the sake of eras payouts.
        fn elect() -> Result<Supports<T::AccountId>, Self::Error> {
            let staking_validators = Self::DataProvider::electable_targets(None)
                .map_err(Self::Error::DataProvider)?
                .into_iter()
                .collect::<BTreeSet<_>>();
            let reserved_validators = NextEraReservedValidators::<T>::get()
                .into_iter()
                .collect::<BTreeSet<_>>();
            let non_reserved_validators = NextEraNonReservedValidators::<T>::get()
                .into_iter()
                .collect::<BTreeSet<_>>();

            let eligible_validators =
                &(&reserved_validators | &non_reserved_validators) & &staking_validators;
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

            Ok(supports.into_iter().collect())
        }
    }
}
