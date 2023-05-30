use codec::{Decode, Encode};
use frame_election_provider_support::Weight;
use frame_support::{
    log,
    pallet_prelude::{StorageVersion, TypeInfo},
    traits::OnRuntimeUpgrade,
};
use primitives::CommitteeSeats;
use sp_core::Get;
#[cfg(feature = "try-runtime")]
use {frame_support::ensure, pallets_support::ensure_storage_version, sp_std::vec::Vec};

use crate::{CommitteeSize, Config, NextEraCommitteeSize, Pallet, LOG_TARGET};

pub mod v4 {
    use super::*;

    /// Migrate storage version to 4.
    pub struct Migration<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for Migration<T> {
        fn on_runtime_upgrade() -> Weight {
            if StorageVersion::get::<Pallet<T>>() != StorageVersion::new(3) {
                log::info!(
                    target: LOG_TARGET,
                    "Skipping migrations from STORAGE_VERSION 3 to 4 for pallet elections"
                );
                return T::DbWeight::get().reads(1);
            };

            log::info!(
                target: LOG_TARGET,
                "Running migration from STORAGE_VERSION 3 to 4 for pallet elections"
            );

            let reads = 1;
            let writes = 1;
            StorageVersion::new(4).put::<Pallet<T>>();
            T::DbWeight::get().reads(reads) + T::DbWeight::get().writes(writes)
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
            ensure_storage_version::<Pallet<T>>(3)?;

            Ok(Vec::new())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(_: Vec<u8>) -> Result<(), &'static str> {
            ensure_storage_version::<Pallet<T>>(4)
        }
    }
}

pub mod v5 {
    use super::*;
    // V4 CommitteeSeats
    #[derive(Decode, Encode, TypeInfo, Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CommitteeSeatsV4 {
        /// Size of reserved validators in a session
        pub reserved_seats: u32,
        /// Size of non reserved validators in a session
        pub non_reserved_seats: u32,
    }

    /// Migration add field for `CommitteeSize` and `NextEraCommitteeSize` `finality_committee_non_reserved_seats` to
    /// `CommitteeSeats`.
    pub struct Migration<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for Migration<T> {
        fn on_runtime_upgrade() -> Weight {
            if StorageVersion::get::<Pallet<T>>() != StorageVersion::new(4) {
                log::info!(
                    target: LOG_TARGET,
                    "Skipping migrations from STORAGE_VERSION 4 to 5 for pallet elections"
                );
                return T::DbWeight::get().reads(1);
            };

            log::info!(
                target: LOG_TARGET,
                "Running migration from STORAGE_VERSION 4 to 5 for pallet elections"
            );

            let reads = 2;
            let mut writes = 1;

            if CommitteeSize::<T>::translate::<CommitteeSeatsV4, _>(|old| {
                if let Some(CommitteeSeatsV4 {
                    reserved_seats,
                    non_reserved_seats,
                }) = old
                {
                    Some(CommitteeSeats {
                        reserved_seats,
                        non_reserved_seats,
                        non_reserved_finality_seats: non_reserved_seats,
                    })
                } else {
                    None
                }
            })
            .is_ok()
            {
                writes += 1;
            } else {
                log::error!(target: LOG_TARGET, "Could not migrate CommitteeSize");
            }

            if NextEraCommitteeSize::<T>::translate::<CommitteeSeatsV4, _>(|old| {
                if let Some(CommitteeSeatsV4 {
                    reserved_seats,
                    non_reserved_seats,
                }) = old
                {
                    Some(CommitteeSeats {
                        reserved_seats,
                        non_reserved_seats,
                        non_reserved_finality_seats: non_reserved_seats,
                    })
                } else {
                    None
                }
            })
            .is_ok()
            {
                writes += 1;
            } else {
                log::error!(target: LOG_TARGET, "Could not migrate NextCommitteeSize");
            }

            StorageVersion::new(5).put::<Pallet<T>>();
            T::DbWeight::get().reads(reads) + T::DbWeight::get().writes(writes)
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
            ensure_storage_version::<Pallet<T>>(4)?;

            Ok(Vec::new())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(_: Vec<u8>) -> Result<(), &'static str> {
            ensure_storage_version::<Pallet<T>>(5)?;

            let committee_seats = CommitteeSize::<T>::get();
            ensure!(
                committee_seats.non_reserved_finality_seats == committee_seats.non_reserved_seats,
                "non_reserved_finality_seats should be equal to non_reserved_seats"
            );
            let committee_seats = NextEraCommitteeSize::<T>::get();
            ensure!(
                committee_seats.non_reserved_finality_seats == committee_seats.non_reserved_seats,
                "non_reserved_finality_seats should be equal to non_reserved_seats"
            );

            Ok(())
        }
    }
}
