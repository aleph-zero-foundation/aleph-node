#[cfg(feature = "try-runtime")]
use frame_support::ensure;
use frame_support::{
    log, storage_alias,
    traits::{Get, OnRuntimeUpgrade, PalletInfoAccess, StorageVersion},
    weights::Weight,
};
#[cfg(feature = "try-runtime")]
use pallets_support::ensure_storage_version;
use pallets_support::StorageMigration;
use sp_std::vec::Vec;

use crate::{
    compute_validator_scaled_total_rewards,
    migrations::Validators,
    traits::{EraInfoProvider, ValidatorRewardsHandler},
    Config, ValidatorEraTotalReward, ValidatorTotalRewards,
};

#[storage_alias]
type Members<T> = StorageValue<Elections, Validators<T>>;
#[storage_alias]
type MembersPerSession = StorageValue<Elections, u32>;
#[storage_alias]
type ReservedMembers<T> = StorageValue<Elections, Validators<T>>;
#[storage_alias]
type NonReservedMembers<T> = StorageValue<Elections, Validators<T>>;
#[storage_alias]
type ErasMembers<T> = StorageValue<Elections, (Validators<T>, Validators<T>)>;

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

impl<T: Config, P: PalletInfoAccess> StorageMigration for Migration<T, P> {
    #[cfg(feature = "try-runtime")]
    const MIGRATION_STORAGE_PREFIX: &'static [u8] = b"PALLET_ELECTIONS::V0_TO_V1_MIGRATION";
}

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
        ensure_storage_version::<P>(0)?;

        let members = Members::<T>::get().ok_or("No `Members` storage")?;
        Self::store_temp("members", members);

        Ok(())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade() -> Result<(), &'static str> {
        ensure_storage_version::<P>(1)?;

        let mps = MembersPerSession::get().ok_or("No `MembersPerSession` in the storage")?;
        let reserved_members =
            ReservedMembers::<T>::get().ok_or("No `ReservedMembers` in the storage")?;
        let non_reserved_members =
            NonReservedMembers::<T>::get().ok_or("No `NonReservedMembers` in the storage")?;
        let eras_members = ErasMembers::<T>::get().ok_or("No `ErasMembers` in the storage")?;

        let old_members = Self::read_temp::<Validators<T>>("members");

        ensure!(
            reserved_members == old_members,
            "Mismatch between `ReservedMembers` and old `Members`"
        );
        ensure!(
            mps as usize == reserved_members.len(),
            "Bad size of the `MembersPerSession`"
        );
        ensure!(
            reserved_members == eras_members.0,
            "Mismatch between `ReservedMembers` and `ErasMembers`"
        );
        ensure!(
            non_reserved_members == eras_members.1,
            "Mismatch between `NonReservedMembers` and `ErasMembers`"
        );
        ensure!(
            non_reserved_members.is_empty(),
            "`NonReservedMembers` should be empty"
        );

        Ok(())
    }
}
