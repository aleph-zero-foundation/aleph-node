use frame_election_provider_support::Weight;
use frame_support::{log, pallet_prelude::StorageVersion, traits::OnRuntimeUpgrade};
use sp_core::Get;
#[cfg(feature = "try-runtime")]
use {pallets_support::ensure_storage_version, sp_std::vec::Vec};

use crate::{Config, Pallet, LOG_TARGET};

/// Migrate storage version to 4.
pub struct Migration<T>(sp_std::marker::PhantomData<T>);

impl<T: Config> OnRuntimeUpgrade for Migration<T> {
    fn on_runtime_upgrade() -> Weight {
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
