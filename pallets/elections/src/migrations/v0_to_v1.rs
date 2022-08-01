use frame_support::{
    log, storage_alias,
    traits::{Get, OnRuntimeUpgrade, PalletInfoAccess, StorageVersion},
    weights::Weight,
};
use sp_std::vec::Vec;

use crate::{
    compute_validator_scaled_total_rewards,
    migrations::StorageMigration,
    traits::{EraInfoProvider, ValidatorRewardsHandler},
    Config, ValidatorEraTotalReward, ValidatorTotalRewards,
};

#[storage_alias]
type Members<T> = StorageValue<Elections, Vec<<T as frame_system::Config>::AccountId>>;
#[storage_alias]
pub type MembersPerSession = StorageValue<Elections, u32>;
#[storage_alias]
type ReservedMembers<T> = StorageValue<Elections, Vec<<T as frame_system::Config>::AccountId>>;
#[storage_alias]
type NonReservedMembers<T> = StorageValue<Elections, Vec<<T as frame_system::Config>::AccountId>>;
#[storage_alias]
type ErasMembers<T> = StorageValue<
    Elections,
    (
        Vec<<T as frame_system::Config>::AccountId>,
        Vec<<T as frame_system::Config>::AccountId>,
    ),
>;

/// The assumptions made by this migration:
///
/// There is one storage in the pallet elections `Members` containing current set of validators.
/// After migration the state should be as follows:
/// - `Members` are no longer present.
/// - `MembersPerSession` is u32 storage set to size of the `Members` set.
/// - `ReservedMembers` contains the content of the `Members`
/// - `NonReservedMembers` are empty
/// - `ErasMembers` contain tuple of (content of `Members`, empty vector).
pub struct Migration<T, P>(sp_std::marker::PhantomData<(T, P)>);

impl<T: Config, P: PalletInfoAccess> OnRuntimeUpgrade for Migration<T, P> {
    fn on_runtime_upgrade() -> Weight {
        log::info!(target: "pallet_elections", "Running migration from STORAGE_VERSION 0 to 1 for pallet elections");

        let members = match Members::<T>::get() {
            Some(m) => m,
            None => {
                log::error!(target: "pallet_elections", "Migration failed, no Members storage");
                return T::DbWeight::get().reads(1);
            }
        };

        Members::<T>::kill();

        let mut writes = 5;
        let mut reads = 2;

        if let Some(era) = T::EraInfoProvider::active_era() {
            let t = T::ValidatorRewardsHandler::validator_totals(era);
            let st = compute_validator_scaled_total_rewards(t);

            ValidatorEraTotalReward::<T>::put(ValidatorTotalRewards(st.into_iter().collect()));

            writes += 1;
            reads += 1;
        }

        let members_per_session = members.len() as u32;

        MembersPerSession::put(members_per_session);
        ReservedMembers::<T>::put(members.clone());
        NonReservedMembers::<T>::put(Vec::<T::AccountId>::new());
        ErasMembers::<T>::put((members, Vec::<T::AccountId>::new()));

        StorageVersion::new(1).put::<P>();
        T::DbWeight::get().reads(reads) + T::DbWeight::get().writes(writes)
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<(), &'static str> {
        match Members::<T>::get() {
            Some(_) => {}
            _ => return Err("No `Members` storage"),
        }

        if StorageVersion::get::<P>() == StorageVersion::new(0) {
            Ok(())
        } else {
            Err("Bad storage version")
        }
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade() -> Result<(), &'static str> {
        let mps = match MembersPerSession::get() {
            Some(mps) => mps,
            _ => return Err("No `MembersPerSession` in the storage"),
        };
        let reserved_members = match ReservedMembers::<T>::get() {
            Some(rm) => rm,
            _ => return Err("No `ReservedMembers` in the storage"),
        };
        let non_reserved_members = match NonReservedMembers::<T>::get() {
            Some(nrm) => nrm,
            _ => return Err("No `NonReservedMembers` in the storage"),
        };
        let eras_members = match ErasMembers::<T>::get() {
            Some(em) => em,
            _ => return Err("No `ErasMembers` in the storage"),
        };

        if mps as usize != reserved_members.len() {
            return Err("Bad size of the `MembersPerSession`");
        }

        if reserved_members != eras_members.0 {
            return Err("Bad contents of the `ReservedMembers` and `ErasMembers`");
        }

        if non_reserved_members != eras_members.1 {
            return Err("Bad contents of the `NonReservedMembers` and `ErasMembers`");
        }

        if !non_reserved_members.is_empty() {
            return Err("`NonReservedMembers` should be empty");
        }

        if StorageVersion::get::<P>() == StorageVersion::new(1) {
            Ok(())
        } else {
            Err("Bad storage version")
        }
    }
}

impl<T: Config, P: PalletInfoAccess> StorageMigration for Migration<T, P> {}
