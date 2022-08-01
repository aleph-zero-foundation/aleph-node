use frame_support::{
    log, storage_alias,
    traits::{Get, OnRuntimeUpgrade, PalletInfoAccess, StorageVersion},
    weights::Weight,
};
use primitives::CommitteeSeats;
use sp_std::vec::Vec;

use crate::{migrations::StorageMigration, Config, EraValidators};

// V2 storages
#[storage_alias]
type CurrentEraValidators<T> =
    StorageValue<Elections, EraValidators<<T as frame_system::Config>::AccountId>>;
#[storage_alias]
type NextEraReservedValidators<T> =
    StorageValue<Elections, Vec<<T as frame_system::Config>::AccountId>>;

// V3 storages
#[storage_alias]
type CommitteeSize = StorageValue<Elections, CommitteeSeats>;
#[storage_alias]
type NextEraCommitteeSize = StorageValue<Elections, CommitteeSeats>;

pub struct Migration<T, P>(sp_std::marker::PhantomData<(T, P)>);

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
        if StorageVersion::get::<P>() == StorageVersion::new(2) {
            Ok(())
        } else {
            Err("Bad storage version")
        }
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade() -> Result<(), &'static str> {
        if let Some(CommitteeSeats { reserved_seats, .. }) = CommitteeSize::get() {
            if let Some(EraValidators { reserved, .. }) = CurrentEraValidators::<T>::get() {
                assert_eq!(
                    reserved.len(),
                    reserved_seats as usize,
                    "Reserved seats should be set to reserved set size"
                );
            } else {
                return Err("No era validators present");
            }
        } else {
            return Err("CommitteeSize storage empty");
        }

        if let Some(CommitteeSeats { reserved_seats, .. }) = NextEraCommitteeSize::get() {
            if let Some(reserved) = NextEraReservedValidators::<T>::get() {
                assert_eq!(
                    reserved.len(),
                    reserved_seats as usize,
                    "Reserved seats should be set to reserved set size"
                );
            } else {
                return Err("No next era validators present");
            }
        } else {
            return Err("NextEraCommitteeSize storage empty");
        }

        if StorageVersion::get::<P>() == StorageVersion::new(3) {
            Ok(())
        } else {
            Err("Bad storage version")
        }
    }
}

impl<T: Config, P: PalletInfoAccess> StorageMigration for Migration<T, P> {}
