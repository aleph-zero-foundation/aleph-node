use crate::Config;
use frame_support::log;
use frame_support::{
    traits::{Get, GetStorageVersion, PalletInfoAccess, StorageVersion},
    weights::Weight,
};
use sp_std::prelude::*;

frame_support::generate_storage_alias!(
    Aleph, SessionForValidatorsChange => Value<Option<u32>>
);

frame_support::generate_storage_alias!(
    Aleph, Validators<T: Config> => Value<Option<Vec<T::AccountId>>>
);

pub fn migrate<T: Config, P: GetStorageVersion + PalletInfoAccess>() -> Weight {
    let on_chain_storage_version = <P as GetStorageVersion>::on_chain_storage_version();
    let current_storage_version = <P as GetStorageVersion>::current_storage_version();

    if on_chain_storage_version == StorageVersion::default() && current_storage_version == 1 {
        log::info!(target: "pallet_aleph", "Running migration from STORAGE_VERSION 0 to 1");

        let mut writes = 0;
        match SessionForValidatorsChange::translate(sp_std::convert::identity) {
            Ok(_) => {
                writes += 1;
                log::info!(target: "pallet_aleph", "Succesfully migrated storage for SessionForValidatorsChange");
            }
            Err(why) => {
                log::error!(target: "pallet_aleph", "Something went wrong during the migration of SessionForValidatorsChange {:?}", why);
            }
        };

        match Validators::<T>::translate(sp_std::convert::identity) {
            Ok(_) => {
                writes += 1;
                log::info!(target: "pallet_aleph", "Succesfully migrated storage for Validators");
            }
            Err(why) => {
                log::error!(target: "pallet_aleph", "Something went wrong during the migration of Validators storage {:?}", why);
            }
        };

        // store new version
        StorageVersion::new(1).put::<P>();
        writes += 1;

        T::DbWeight::get().reads(3) + T::DbWeight::get().writes(writes)
    } else {
        log::warn!(
            target: "pallet_aleph",
            "Not applying any storage migration because on-chain storage version is {:?} and the version declared in the aleph pallet is {:?}",
            on_chain_storage_version,
            current_storage_version
        );
        // I have only read the version
        T::DbWeight::get().reads(1)
    }
}
