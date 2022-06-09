use crate::Config;
use frame_support::{
    log, storage_alias,
    traits::{Get, PalletInfoAccess, StorageVersion},
    weights::Weight,
};

#[storage_alias]
type SessionForValidatorsChange = StorageValue<Aleph, ()>;

#[storage_alias]
type MillisecsPerBlock = StorageValue<Aleph, ()>;

#[storage_alias]
type SessionPeriod = StorageValue<Aleph, ()>;

#[storage_alias]
type Validators = StorageValue<Aleph, ()>;

pub fn migrate<T: Config, P: PalletInfoAccess>() -> Weight {
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
