//! This pallet manages changes in the committee responsible for producing blocks and establishing consensus.
//! Currently, it's PoA where the validators are set by the root account. In the future, a new
//! version for DPoS elections will replace the current one.
//!
//! ### Terminology
//! For definition of session, era, staking see pallet_session and pallet_staking.
//! - Committee: Set of nodes that produce and finalize blocks in the era.
//! - Validator: Node that can become a member of committee (or already is) via rotation.
//! - (TODO: remove this to remove confusion) Member: Usually same as validator, sometimes means member of the committee
//! - ReservedMembers: Validators that are chosen to be in committee every single session.
//!
//! ### Storage
//! - `Members` - List of possible validators.
//! - `MembersPerSession` - Committee size.
//! - `ReservedMembers` - List of reserved nodes.
//! - `ErasReserved` - List of reserved nodes for the current era.
//!   This is populated from `ReservedMembers` at the time of planning the first session of the era.
//! - `SessionValidatorBlockCount` - Count per validator, how many blocks did the validator produced
//!   in the current session.
//! - `ValidatorEraTotalReward` - Total possible reward per validator for the current era. Scaled to
//!   fit in the u32.

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
use scale_info::TypeInfo;
use sp_std::{
    collections::{btree_map::BTreeMap, btree_set::BTreeSet},
    prelude::Vec,
};

pub use impls::compute_validator_scaled_total_rewards;
pub use pallet::*;

const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

pub type BlockCount = u32;
pub type TotalReward = u32;

#[derive(Decode, Encode, TypeInfo)]
pub struct ValidatorTotalRewards<T>(pub BTreeMap<T, TotalReward>);

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use crate::traits::{EraInfoProvider, SessionInfoProvider, ValidatorRewardsHandler};
    use frame_election_provider_support::{
        ElectionDataProvider, ElectionProvider, Support, Supports,
    };
    use frame_support::{log, pallet_prelude::*, traits::Get};
    use frame_system::{
        ensure_root,
        pallet_prelude::{BlockNumberFor, OriginFor},
    };
    use pallet_session::SessionManager;
    use primitives::DEFAULT_MEMBERS_PER_SESSION;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Something that provides information about ongoing eras.
        type EraInfoProvider: EraInfoProvider;
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
        ChangeMembers(Vec<T::AccountId>, Vec<T::AccountId>, u32),
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
                        migrations::v0_to_v1::migrate::<T, Self>()
                    }
                    _ => {
                        log::warn!(
                            target: "pallet_elections",
                            "On chain storage version of pallet elections is {:?} but it should not be bigger than 1",
                            on_chain
                        );
                        0
                    }
                }
        }
    }

    #[pallet::storage]
    #[pallet::getter(fn non_reserved_members)]
    pub type NonReservedMembers<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

    #[pallet::storage]
    pub type MembersPerSession<T> = StorageValue<_, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn reserved_members)]
    pub type ReservedMembers<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

    #[pallet::storage]
    pub type ErasMembers<T: Config> =
        StorageValue<_, (Vec<T::AccountId>, Vec<T::AccountId>), ValueQuery>;

    #[pallet::storage]
    pub type SessionValidatorBlockCount<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, BlockCount, ValueQuery>;

    #[pallet::storage]
    pub type ValidatorEraTotalReward<T: Config> =
        StorageValue<_, ValidatorTotalRewards<T::AccountId>, OptionQuery>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight((T::BlockWeights::get().max_block, DispatchClass::Operational))]
        pub fn change_members(
            origin: OriginFor<T>,
            reserved_members: Option<Vec<T::AccountId>>,
            non_reserved_members: Option<Vec<T::AccountId>>,
            members_per_session: Option<u32>,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let mps = members_per_session.unwrap_or_else(MembersPerSession::<T>::get);
            let reserved_members = reserved_members.unwrap_or_else(ReservedMembers::<T>::get);
            let non_reserved_members =
                non_reserved_members.unwrap_or_else(NonReservedMembers::<T>::get);

            Self::ensure_members_are_ok(
                reserved_members.clone(),
                non_reserved_members.clone(),
                mps,
            )?;

            NonReservedMembers::<T>::put(non_reserved_members.clone());
            ReservedMembers::<T>::put(reserved_members.clone());
            MembersPerSession::<T>::put(mps);

            Self::deposit_event(Event::ChangeMembers(
                reserved_members,
                non_reserved_members,
                mps,
            ));

            Ok(())
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub non_reserved_members: Vec<T::AccountId>,
        pub reserved_members: Vec<T::AccountId>,
        pub members_per_session: u32,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                non_reserved_members: Vec::new(),
                reserved_members: Vec::new(),
                members_per_session: DEFAULT_MEMBERS_PER_SESSION,
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            <NonReservedMembers<T>>::put(&self.non_reserved_members);
            <MembersPerSession<T>>::put(&self.members_per_session);
            <ReservedMembers<T>>::put(&self.reserved_members);
        }
    }

    impl<T: Config> Pallet<T> {
        fn ensure_members_are_ok(
            reserved_members: Vec<T::AccountId>,
            non_reserved_members: Vec<T::AccountId>,
            members_per_session: u32,
        ) -> DispatchResult {
            let reserved_len = reserved_members.len() as u32;
            let non_reserved_len = non_reserved_members.len() as u32;
            let members_size = reserved_len + non_reserved_len;

            ensure!(
                members_per_session >= reserved_len,
                Error::<T>::TooManyReservedMembers
            );
            ensure!(
                members_per_session <= members_size,
                Error::<T>::NotEnoughMembers
            );

            let member_set: BTreeSet<_> = reserved_members
                .into_iter()
                .chain(non_reserved_members.into_iter())
                .collect();

            ensure!(
                member_set.len() as u32 == members_size,
                Error::<T>::NonUniqueListOfMembers
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
        TooManyReservedMembers,
        NotEnoughMembers,
        NonUniqueListOfMembers,
    }

    impl<T: Config> ElectionProvider for Pallet<T> {
        type AccountId = T::AccountId;
        type BlockNumber = T::BlockNumber;
        type Error = ElectionError;
        type DataProvider = T::DataProvider;

        // The elections are PoA so only the nodes listed in the Members will be elected as validators.
        // We calculate the supports for them for the sake of eras payouts.
        fn elect() -> Result<Supports<T::AccountId>, Self::Error> {
            let voters =
                Self::DataProvider::electing_voters(None).map_err(Self::Error::DataProvider)?;
            let members = Pallet::<T>::non_reserved_members()
                .into_iter()
                .chain(Pallet::<T>::reserved_members().into_iter());
            let mut supports: BTreeMap<_, _> = members
                .map(|id| {
                    (
                        id,
                        Support {
                            total: 0,
                            voters: Vec::new(),
                        },
                    )
                })
                .collect();

            for (voter, vote, targets) in voters {
                // The parameter Staking::MAX_NOMINATIONS is set to 1 which guarantees that len(targets) == 1
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
