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
use primitives::CommitteeSeats;

use crate::{migrations::Validators, Config, EraValidators};

// V2 storages
#[storage_alias]
type CurrentEraValidators<T> =
    StorageValue<Elections, EraValidators<<T as frame_system::Config>::AccountId>>;
#[storage_alias]
type NextEraReservedValidators<T> = StorageValue<Elections, Validators<T>>;

// V3 storages
#[storage_alias]
type CommitteeSize = StorageValue<Elections, CommitteeSeats>;
#[storage_alias]
type NextEraCommitteeSize = StorageValue<Elections, CommitteeSeats>;

/// Migration changes type for `CommitteeSize` and `NextEraCommitteeSize` from `u32` to
/// `CommitteeSeats`.
pub struct Migration<T, P>(sp_std::marker::PhantomData<(T, P)>);

impl<T: Config, P: PalletInfoAccess> StorageMigration for Migration<T, P> {
    #[cfg(feature = "try-runtime")]
    const MIGRATION_STORAGE_PREFIX: &'static [u8] = b"PALLET_ELECTIONS::V2_TO_V3_MIGRATION";
}

impl<T: Config, P: PalletInfoAccess> OnRuntimeUpgrade for Migration<T, P> {
    fn on_runtime_upgrade() -> Weight {
        log::info!(target: "pallet_elections", "Running migration from STORAGE_VERSION 2 to 3 for pallet elections");

        let mut reads = 2;
        let mut writes = 1;

        if let Some(EraValidators { reserved, .. }) = CurrentEraValidators::<T>::get() {
            let reserved_len = reserved.len();
            reads += 1;
            match CommitteeSize::translate::<u32, _>(|old: Option<u32>| {
                Some(match old {
                    Some(cs) => CommitteeSeats {
                        reserved_seats: reserved_len as u32,
                        non_reserved_seats: cs.saturating_sub(reserved_len as u32),
                    },
                    None => CommitteeSeats {
                        reserved_seats: reserved_len as u32,
                        non_reserved_seats: 0,
                    },
                })
            }) {
                Ok(_) => {
                    writes += 1;
                    log::info!(target: "pallet_elections", "Successfully migrated storage for CommitteeSize");
                }
                Err(why) => {
                    log::error!(target: "pallet_elections", "Something went wrong during the migration of CommitteeSize storage {:?}", why);
                }
            }
        }

        if let Some(reserved) = NextEraReservedValidators::<T>::get() {
            let n_era_reserved_len = reserved.len();
            reads += 1;
            match NextEraCommitteeSize::translate::<u32, _>(|old| {
                Some(match old {
                    Some(cs) => CommitteeSeats {
                        reserved_seats: n_era_reserved_len as u32,
                        non_reserved_seats: cs.saturating_sub(n_era_reserved_len as u32),
                    },
                    None => CommitteeSeats {
                        reserved_seats: n_era_reserved_len as u32,
                        non_reserved_seats: 0,
                    },
                })
            }) {
                Ok(_) => {
                    writes += 1;
                    log::info!(target: "pallet_elections", "Successfully migrated storage for NextEraCommitteeSize");
                }
                Err(why) => {
                    log::error!(target: "pallet_elections", "Something went wrong during the migration of NextEraCommitteeSize storage {:?}", why);
                }
            }
        }

        StorageVersion::new(3).put::<P>();

        T::DbWeight::get().reads(reads) + T::DbWeight::get().writes(writes)
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<(), &'static str> {
        #[storage_alias]
        type CommitteeSize = StorageValue<Elections, u32>;
        #[storage_alias]
        type NextEraCommitteeSize = StorageValue<Elections, u32>;

        ensure_storage_version::<P>(2)?;

        let committee_size = CommitteeSize::get();
        Self::store_temp("committee_size", committee_size);

        let next_era_committee_size = NextEraCommitteeSize::get();
        Self::store_temp("next_era_committee_size", next_era_committee_size);

        Ok(())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade() -> Result<(), &'static str> {
        ensure_storage_version::<P>(3)?;

        let new_committee_size = CommitteeSize::get().ok_or("No `CommitteeSize` in the storage")?;
        let new_next_era_committee_size =
            NextEraCommitteeSize::get().ok_or("No `NextEraCommitteeSize` in the storage")?;
        // The next two are exactly the same as before migration.
        let current_era_validators =
            CurrentEraValidators::<T>::get().ok_or("No `CurrentEraValidators` in the storage")?;
        let next_era_reserved_validators = NextEraReservedValidators::<T>::get()
            .ok_or("No `NextEraReservedValidators` in the storage")?;

        let old_committee_size =
            Self::read_temp::<Option<u32>>("committee_size").unwrap_or_default();
        let old_next_era_committee_size =
            Self::read_temp::<Option<u32>>("next_era_committee_size").unwrap_or_default();

        let currently_reserved = current_era_validators.reserved.len();
        ensure!(
            new_committee_size.reserved_seats == currently_reserved as u32,
            "Mismatch between `CurrentEraValidators` and `CommitteeSize`"
        );
        ensure!(
            new_committee_size.non_reserved_seats
                == old_committee_size.saturating_sub(currently_reserved as u32),
            "Mismatch between `CurrentEraValidators` and `CommitteeSize`"
        );

        let next_reserved = next_era_reserved_validators.len();
        ensure!(
            new_next_era_committee_size.reserved_seats == next_reserved as u32,
            "Mismatch between `NextEraReservedValidators` and `NextEraCommitteeSize`"
        );
        ensure!(
            new_next_era_committee_size.non_reserved_seats
                == old_next_era_committee_size.saturating_sub(next_reserved as u32),
            "Mismatch between `NextEraReservedValidators` and `NextEraCommitteeSize`"
        );

        Ok(())
    }
}
