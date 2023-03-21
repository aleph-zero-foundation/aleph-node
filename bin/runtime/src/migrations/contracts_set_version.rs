use frame_support::{
    log,
    pallet_prelude::*,
    traits::{Get, OnRuntimeUpgrade},
    weights::Weight,
};
use pallet_contracts::{Config, Pallet};
use sp_std::marker::PhantomData;
#[cfg(feature = "try-runtime")]
use sp_std::vec::Vec;

const TARGET: &str = "runtime::custom_contract_migration";

pub struct ContractsSetVersion9<T: Config>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for ContractsSetVersion9<T> {
    fn on_runtime_upgrade() -> Weight {
        let version = StorageVersion::get::<Pallet<T>>();
        let mut weight = T::DbWeight::get().reads_writes(1, 0);
        log::info!(
            target: TARGET,
            "On-chain version of pallet contracts is {:?}",
            version
        );
        if version < 9 {
            weight += T::DbWeight::get().reads_writes(0, 1);
            StorageVersion::new(9).put::<Pallet<T>>();
            log::info!(
                target: TARGET,
                "Setting the version of pallet contracts to 9."
            );
        }

        weight
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
        let version = StorageVersion::get::<Pallet<T>>();
        log::warn!(
            target: TARGET,
            "Pre-upgrade version in custom contracts migration: {:?}",
            version
        );
        if version != 0 {
            return Err("Version should be 0, because pallet contracts is not present.");
        }

        Ok(Vec::new())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(_: Vec<u8>) -> Result<(), &'static str> {
        let version = StorageVersion::get::<Pallet<T>>();
        log::warn!(
            target: TARGET,
            "Post-upgrade version in custom contracts migration: {:?}",
            version
        );
        if version != StorageVersion::new(9) {
            return Err("Version should be 9 after migration.");
        }
        Ok(())
    }
}
