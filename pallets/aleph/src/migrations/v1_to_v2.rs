use crate::Config;
use frame_support::{
    generate_storage_alias, log,
    traits::{Get, PalletInfoAccess, StorageVersion},
    weights::Weight,
};

generate_storage_alias!(Aleph, SessionForValidatorsChange => Value<()>);
generate_storage_alias!(Aleph, MillisecsPerBlock => Value<()>);
generate_storage_alias!(Aleph, SessionPeriod => Value<()>);
generate_storage_alias!(Aleph, Validators => Value<()>);

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
