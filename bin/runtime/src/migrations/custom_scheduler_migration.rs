#[cfg(feature = "try-runtime")]
use codec::{Decode, Encode};
use frame_support::{
    log,
    pallet_prelude::{Get, StorageVersion},
    traits::OnRuntimeUpgrade,
};
use pallet_scheduler::{Agenda, Config, Pallet};
#[cfg(feature = "try-runtime")]
use sp_std::vec::Vec;

use crate::Weight;

const TARGET: &str = "runtime::scheduler::migration";

/// Custom migrations the scheduler pallet from V0 to V3 that only bumps StorageVersion to 3
pub struct MigrateToV3<T>(sp_std::marker::PhantomData<T>);

impl<T: Config> OnRuntimeUpgrade for MigrateToV3<T> {
    fn on_runtime_upgrade() -> Weight {
        let version = StorageVersion::get::<Pallet<T>>();
        if version != 0 {
            log::warn!(
                target: TARGET,
                "skipping v0 to v3 migration: executed on wrong storage version.\
				Expected version 0, found {:?}",
                version,
            );
            return T::DbWeight::get().reads(1);
        }

        let agendas = Agenda::<T>::iter_keys().count() as u32;
        if agendas != 0 {
            log::warn!(
                target: TARGET,
                "skipping v0 to v3 migration: Agendas are not empty. Found {:?} agendas.",
                agendas,
            );
            return T::DbWeight::get().reads(1 + agendas as u64);
        }

        StorageVersion::new(3).put::<Pallet<T>>();
        T::DbWeight::get().reads_writes(1 + agendas as u64, 1)
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
        assert_eq!(
            StorageVersion::get::<Pallet<T>>(),
            0,
            "Can only upgrade from version 0"
        );

        let agendas = Agenda::<T>::iter_keys().count() as u32;
        assert_eq!(agendas, 0, "Agendas should be empty pre-upgrade!");

        Ok(agendas.encode())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(state: Vec<u8>) -> Result<(), &'static str> {
        assert_eq!(StorageVersion::get::<Pallet<T>>(), 3, "Must upgrade");

        let old_agendas: u32 =
            Decode::decode(&mut &state[..]).expect("pre_upgrade provides a valid state; qed");
        assert_eq!(old_agendas, 0, "Agendas should be empty pre-upgrade!");

        let new_agendas = Agenda::<T>::iter_keys().count() as u32;
        assert_eq!(new_agendas, 0, "Agendas should be empty post-upgrade!");

        log::info!(
            target: TARGET,
            "Migrated 0 agendas, bumped StorageVersion to V3"
        );

        Ok(())
    }
}
