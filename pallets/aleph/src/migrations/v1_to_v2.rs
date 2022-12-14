use frame_support::{
    log, storage_alias,
    traits::{Get, OnRuntimeUpgrade, PalletInfoAccess, StorageVersion},
    weights::Weight,
};
#[cfg(feature = "try-runtime")]
use {frame_support::ensure, pallets_support::ensure_storage_version, sp_std::vec::Vec};

use crate::Config;

#[storage_alias]
type SessionForValidatorsChange = StorageValue<Aleph, ()>;

#[storage_alias]
type MillisecsPerBlock = StorageValue<Aleph, ()>;

#[storage_alias]
type SessionPeriod = StorageValue<Aleph, ()>;

#[storage_alias]
type Validators = StorageValue<Aleph, ()>;

/// Removes:
///   - SessionForValidatorsChange
///   - MillisecsPerBlock
///   - SessionPeriod
///   - Validators
pub struct Migration<T, P>(sp_std::marker::PhantomData<(T, P)>);

impl<T: Config, P: PalletInfoAccess> OnRuntimeUpgrade for Migration<T, P> {
    fn on_runtime_upgrade() -> Weight {
        let mut writes = 0;
        let mut reads = 0;
        log::info!(target: "pallet_aleph", "Running migration from STORAGE_VERSION 1 to 2");

        if !SessionForValidatorsChange::exists() {
            log::info!(target: "pallet_aleph", "Storage item SessionForValidatorsChange does not exist!");
        } else {
            writes += 1;
        }
        SessionForValidatorsChange::kill();
        reads += 1;

        if !MillisecsPerBlock::exists() {
            log::info!(target: "pallet_aleph", "Storage item MillisecsPerBlock does not exist!");
        } else {
            writes += 1;
        }
        MillisecsPerBlock::kill();
        reads += 1;

        if !SessionPeriod::exists() {
            log::info!(target: "pallet_aleph", "Storage item SessionPeriod does not exist!");
        } else {
            writes += 1;
        }
        SessionPeriod::kill();
        reads += 1;

        if !Validators::exists() {
            log::info!(target: "pallet_aleph", "Storage item Validators does not exist!");
        } else {
            writes += 1;
        }
        Validators::kill();
        reads += 1;

        // store new version
        StorageVersion::new(2).put::<P>();
        writes += 1;

        T::DbWeight::get().reads(reads) + T::DbWeight::get().writes(writes)
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
        ensure_storage_version::<P>(1)?;
        Ok(Vec::new())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(_state: Vec<u8>) -> Result<(), &'static str> {
        ensure_storage_version::<P>(2)?;

        ensure!(
            SessionForValidatorsChange::get().is_none(),
            "`SessionForValidatorsChange` should be removed"
        );
        ensure!(
            MillisecsPerBlock::get().is_none(),
            "`MillisecsPerBlock` should be removed"
        );
        ensure!(
            SessionPeriod::get().is_none(),
            "`SessionPeriod` should be removed"
        );
        ensure!(
            Validators::get().is_none(),
            "`Validators` should be removed"
        );

        Ok(())
    }
}
