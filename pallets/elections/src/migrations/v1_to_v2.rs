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

use crate::{migrations::Validators, Config, EraValidators};

// V1 storages
#[storage_alias]
pub type MembersPerSession = StorageValue<Elections, u32>;
#[storage_alias]
type ReservedMembers<T> = StorageValue<Elections, Validators<T>>;
#[storage_alias]
type NonReservedMembers<T> = StorageValue<Elections, Validators<T>>;
#[storage_alias]
type ErasMembers<T> = StorageValue<Elections, (Validators<T>, Validators<T>)>;

// V2 storages
#[storage_alias]
type CommitteeSize = StorageValue<Elections, u32>;
#[storage_alias]
type NextEraReservedValidators<T> = StorageValue<Elections, Validators<T>>;
#[storage_alias]
type NextEraNonReservedValidators<T> = StorageValue<Elections, Validators<T>>;
#[storage_alias]
type CurrentEraValidators<T> =
    StorageValue<Elections, EraValidators<<T as frame_system::Config>::AccountId>>;

/// This migration refactor storages as follow:
///
/// - `MembersPerSession` -> `CommitteeSize`
/// - `ReservedMembers` -> `NextEraReservedMembers`
/// - `NonReservedMembers` -> `NextEraNonReservedMembers`
/// - `ErasMembers` `(reserved, non_reserved)` -> `CurrentEraValidators` `ErasValidators { reserved, non_reserved}`
pub struct Migration<T, P>(sp_std::marker::PhantomData<(T, P)>);

impl<T: Config, P: PalletInfoAccess> StorageMigration for Migration<T, P> {
    #[cfg(feature = "try-runtime")]
    const MIGRATION_STORAGE_PREFIX: &'static [u8] = b"PALLET_ELECTIONS::V1_TO_V2_MIGRATION";
}

impl<T: Config, P: PalletInfoAccess> OnRuntimeUpgrade for Migration<T, P> {
    fn on_runtime_upgrade() -> Weight {
        log::info!(target: "pallet_elections", "Running migration from STORAGE_VERSION 1 to 2 for pallet elections");

        let mut writes = 1;
        let reads = 4;

        if let Some(mps) = MembersPerSession::get() {
            CommitteeSize::put(mps);
            writes += 1;
        }
        if let Some(reserved) = ReservedMembers::<T>::get() {
            NextEraReservedValidators::<T>::put(reserved);
            writes += 1;
        }
        if let Some(non_reserved) = NonReservedMembers::<T>::get() {
            NextEraNonReservedValidators::<T>::put(non_reserved);
            writes += 1;
        }
        if let Some((reserved, non_reserved)) = ErasMembers::<T>::get() {
            CurrentEraValidators::<T>::put(EraValidators {
                reserved,
                non_reserved,
            });
            writes += 1;
        }

        MembersPerSession::kill();
        ReservedMembers::<T>::kill();
        NonReservedMembers::<T>::kill();
        ErasMembers::<T>::kill();

        StorageVersion::new(2).put::<P>();
        T::DbWeight::get().reads(reads) + T::DbWeight::get().writes(writes)
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<(), &'static str> {
        ensure_storage_version::<P>(1)?;

        let members_per_session =
            MembersPerSession::get().ok_or("No `MembersPerSession` in the storage")?;
        Self::store_temp("members_per_session", members_per_session);

        let reserved_members =
            ReservedMembers::<T>::get().ok_or("No `ReservedMembers` in the storage")?;
        Self::store_temp("reserved_members", reserved_members);

        let non_reserved_members =
            NonReservedMembers::<T>::get().ok_or("No `NonReservedMembers` in the storage")?;
        Self::store_temp("non_reserved_members", non_reserved_members);

        let eras_members = ErasMembers::<T>::get().ok_or("No `ErasMembers` in the storage")?;
        Self::store_temp("eras_members", eras_members);

        Ok(())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade() -> Result<(), &'static str> {
        ensure_storage_version::<P>(2)?;

        let committee_size = CommitteeSize::get().ok_or("No `CommitteeSize` in the storage")?;
        let next_era_reserved_validators = NextEraReservedValidators::<T>::get()
            .ok_or("No `NextEraReservedValidators` in the storage")?;
        let next_era_non_reserved_validators = NextEraNonReservedValidators::<T>::get()
            .ok_or("No `NextEraNonReservedValidators` in the storage")?;
        let current_era_validators =
            CurrentEraValidators::<T>::get().ok_or("No `CurrentEraValidators` in the storage")?;

        let members_per_session = Self::read_temp::<u32>("members_per_session");
        let reserved_members = Self::read_temp::<Validators<T>>("reserved_members");
        let non_reserved_members = Self::read_temp::<Validators<T>>("non_reserved_members");
        let eras_members = Self::read_temp::<(Validators<T>, Validators<T>)>("eras_members");

        ensure!(
            committee_size == members_per_session,
            "Mismatch between `CommitteeSize` and `MembersPerSession`"
        );
        ensure!(
            next_era_reserved_validators == reserved_members,
            "Mismatch between `NextEraReservedValidators` and `ReservedMembers`"
        );
        ensure!(
            next_era_non_reserved_validators == non_reserved_members,
            "Mismatch between `NextEraNonReservedValidators` and `NonReservedMembers`"
        );
        ensure!(
            current_era_validators.reserved == eras_members.0,
            "Mismatch between `CurrentEraValidators` and `ErasMembers`"
        );
        ensure!(
            current_era_validators.non_reserved == eras_members.1,
            "Mismatch between `CurrentEraValidators` and `ErasMembers`"
        );

        Ok(())
    }
}
