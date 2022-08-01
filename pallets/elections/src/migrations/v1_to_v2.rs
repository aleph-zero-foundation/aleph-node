use frame_support::{
    log, storage_alias,
    traits::{Get, OnRuntimeUpgrade, PalletInfoAccess, StorageVersion},
    weights::Weight,
};
use sp_std::vec::Vec;

use crate::{migrations::StorageMigration, Config, EraValidators};

// V1 storages
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

// V2 storages
#[storage_alias]
type CommitteeSize = StorageValue<Elections, u32>;
#[storage_alias]
type NextEraReservedValidators<T> =
    StorageValue<Elections, Vec<<T as frame_system::Config>::AccountId>>;
#[storage_alias]
type NextEraNonReservedValidators<T> =
    StorageValue<Elections, Vec<<T as frame_system::Config>::AccountId>>;
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
        match MembersPerSession::get() {
            Some(_) => {}
            _ => return Err("No `Members` storage"),
        }
        match ReservedMembers::<T>::get() {
            Some(_) => {}
            _ => return Err("No `Members` storage"),
        }
        match NonReservedMembers::<T>::get() {
            Some(_) => {}
            _ => return Err("No `Members` storage"),
        }
        match ErasMembers::<T>::get() {
            Some(_) => {}
            _ => return Err("No `Members` storage"),
        }

        if StorageVersion::get::<P>() == StorageVersion::new(1) {
            Ok(())
        } else {
            Err("Bad storage version")
        }
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade() -> Result<(), &'static str> {
        match CommitteeSize::get() {
            Some(_) => {}
            _ => return Err("No `CommitteeSize` in the storage"),
        }
        match NextEraReservedValidators::<T>::get() {
            Some(_) => {}
            _ => return Err("No `NextEraReservedValidators` in the storage"),
        }
        match NextEraNonReservedValidators::<T>::get() {
            Some(_) => {}
            _ => return Err("No `NextEraNonReservedValidators` in the storage"),
        };
        match CurrentEraValidators::<T>::get() {
            Some(_) => {}
            _ => return Err("No `CurrentEraValidators` in the storage"),
        };

        if StorageVersion::get::<P>() == StorageVersion::new(2) {
            Ok(())
        } else {
            Err("Bad storage version")
        }
    }
}

impl<T: Config, P: PalletInfoAccess> StorageMigration for Migration<T, P> {}
