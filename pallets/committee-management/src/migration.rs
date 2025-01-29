use frame_support::{
    pallet_prelude::{PalletInfoAccess, StorageVersion, ValueQuery, Weight},
    storage_alias,
    traits::OnRuntimeUpgrade,
};
use log::info;
use parity_scale_codec::Decode;
use primitives::{ProductionBanConfig as ProductionBanConfigStruct, SessionValidators};
use sp_std::vec::Vec;

use crate::{CurrentAndNextSessionValidators, CurrentAndNextSessionValidatorsStorage};

/// Ensure that the current pallet storage version matches `version`.
pub fn ensure_storage_version<P: PalletInfoAccess>(version: u16) -> Result<(), &'static str> {
    if StorageVersion::get::<P>() == StorageVersion::new(version) {
        Ok(())
    } else {
        Err("Bad storage version")
    }
}

pub mod v2 {
    use frame_support::traits::Get;

    use super::*;
    use crate::{Config, Pallet, ProductionBanConfig, LOG_TARGET};

    const OLD_VERSION: u16 = 1;
    const NEW_VERSION: u16 = 2;

    #[derive(Decode)]
    pub struct SessionValidatorsLegacy<T> {
        pub committee: Vec<T>,
        pub non_committee: Vec<T>,
    }

    #[derive(Decode)]
    pub struct CurrentAndNextSessionValidatorsLegacy<T> {
        pub next: SessionValidatorsLegacy<T>,
        pub current: SessionValidatorsLegacy<T>,
    }

    #[storage_alias]
    type BanConfig<T: Config> = StorageValue<Pallet<T>, ProductionBanConfigStruct, ValueQuery>;

    pub struct Migration<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config + pallet_aleph::Config> OnRuntimeUpgrade for Migration<T> {
        fn on_runtime_upgrade() -> Weight {
            if StorageVersion::get::<Pallet<T>>() != StorageVersion::new(OLD_VERSION) {
                log::info!(
                    target: LOG_TARGET,
                    "Skipping migrations from STORAGE_VERSION 1 to 2 for pallet committee-management."
                );
                return T::DbWeight::get().reads(1);
            };

            let reads = 4; // StorageVersion, CurrentAndNextSessionValidatorsStorage, NextFinalityCommittee,  BanConfig
            let mut writes = 2; // StorageVersion, ProductionBanConfig
            info!(target: LOG_TARGET, "Running migration from STORAGE_VERSION 1 to 2 for pallet committee-management.");

            let res = CurrentAndNextSessionValidatorsStorage::<T>::translate::<
                CurrentAndNextSessionValidatorsLegacy<T::AccountId>,
                _,
            >(|current_validators_legacy| {
                let current_validators_legacy =
                    current_validators_legacy.expect("This storage exists");

                let finalizers = pallet_aleph::NextFinalityCommittee::<T>::get();
                let current_validators = SessionValidators {
                    producers: current_validators_legacy.current.committee,
                    finalizers: finalizers.clone(), // we use next finalizers as it's hard to get current but we won't need them in current session.
                    non_committee: current_validators_legacy.current.non_committee,
                };
                let next_validators = SessionValidators {
                    producers: current_validators_legacy.next.committee,
                    finalizers,
                    non_committee: current_validators_legacy.next.non_committee,
                };

                Some(CurrentAndNextSessionValidators {
                    current: current_validators,
                    next: next_validators,
                })
            });
            if res.is_ok() {
                writes += 1;
            } else {
                log::error!(target: LOG_TARGET, "Could not migrate CurrentAndNextSessionValidatorsStorage.");
            };

            let ban_config = BanConfig::<T>::get();
            ProductionBanConfig::<T>::put(ban_config);
            BanConfig::<T>::kill();

            StorageVersion::new(NEW_VERSION).put::<Pallet<T>>();
            info!(target: LOG_TARGET, "Finished migration from STORAGE_VERSION 1 to 2 for pallet committee-management.");

            T::DbWeight::get().reads(reads) + T::DbWeight::get().writes(writes)
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::DispatchError> {
            ensure_storage_version::<Pallet<T>>(OLD_VERSION)?;

            Ok(Vec::new())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(_: Vec<u8>) -> Result<(), sp_runtime::DispatchError> {
            Ok(ensure_storage_version::<Pallet<T>>(NEW_VERSION)?)
        }
    }
}
