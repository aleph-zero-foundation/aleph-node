use frame_support::{
    log::info,
    migration::move_storage_from_pallet,
    pallet_prelude::{Get, PalletInfoAccess, StorageVersion},
    storage::generator::StorageValue,
    traits::OnRuntimeUpgrade,
    weights::Weight,
    StoragePrefixedMap,
};
#[cfg(feature = "try-runtime")]
use {
    crate::{BanConfigStruct, ValidatorTotalRewards},
    codec::{Decode, Encode},
    frame_support::{ensure, traits::STORAGE_VERSION_STORAGE_KEY_POSTFIX},
    pallets_support::ensure_storage_version,
    scale_info::TypeInfo,
    sp_io::hashing::twox_128,
    sp_std::vec::Vec,
};

use crate::{Config, Pallet, LOG_TARGET};

const OLD_PREFIX: &str = "Elections";

#[cfg(feature = "try-runtime")]
mod elections {
    use frame_support::{storage_alias, Twox64Concat};
    use primitives::{BanConfig as BanConfigStruct, BanInfo, BlockCount, SessionCount};

    use crate::ValidatorTotalRewards;

    #[storage_alias]
    pub type SessionValidatorBlockCount<T> =
        StorageMap<Elections, Twox64Concat, <T as frame_system::Config>::AccountId, BlockCount>;
    #[storage_alias]
    pub type ValidatorEraTotalReward<T> =
        StorageValue<Elections, ValidatorTotalRewards<<T as frame_system::Config>::AccountId>>;
    #[storage_alias]
    pub type BanConfig = StorageValue<Elections, BanConfigStruct>;
    #[storage_alias]
    pub type UnderperformedValidatorSessionCount<T> =
        StorageMap<Elections, Twox64Concat, <T as frame_system::Config>::AccountId, SessionCount>;
    #[storage_alias]
    pub type Banned<T> =
        StorageMap<Elections, Twox64Concat, <T as frame_system::Config>::AccountId, BanInfo>;
}

#[cfg(feature = "try-runtime")]
#[derive(Encode, Decode, PartialEq, Eq, TypeInfo)]
struct MigrationStateCheck<AccountId> {
    pub session_validator_block_count: u32,
    pub validator_era_total_reward: Option<ValidatorTotalRewards<AccountId>>,
    pub ban_config: BanConfigStruct,
    pub underperformed_validator_session_count: u32,
    pub banned: u32,
}

/// migrate prefixes from Elections to this pallet.
pub struct PrefixMigration<T>(sp_std::marker::PhantomData<T>);

impl<T: Config> OnRuntimeUpgrade for PrefixMigration<T> {
    fn on_runtime_upgrade() -> Weight {
        if StorageVersion::get::<Pallet<T>>() != StorageVersion::new(0) {
            info!(
                target: LOG_TARGET,
                "Skipping migrations from STORAGE_VERSION 0 to 1 for pallet committee management"
            );
            return T::DbWeight::get().reads(1);
        };

        let pallet_name = Pallet::<T>::name();

        let prefix = crate::SessionValidatorBlockCount::<T>::storage_prefix();
        move_storage_from_pallet(prefix, OLD_PREFIX.as_bytes(), pallet_name.as_bytes());
        info!(target: LOG_TARGET, "Migrated session_validator_block_count");

        let prefix = crate::ValidatorEraTotalReward::<T>::storage_prefix();
        move_storage_from_pallet(prefix, OLD_PREFIX.as_bytes(), pallet_name.as_bytes());
        info!(target: LOG_TARGET, "Migrated validator_era_total_reward");

        let prefix = crate::BanConfig::<T>::storage_prefix();
        move_storage_from_pallet(prefix, OLD_PREFIX.as_bytes(), pallet_name.as_bytes());
        info!(target: LOG_TARGET, "Migrated BanConfig");

        let prefix = crate::UnderperformedValidatorSessionCount::<T>::storage_prefix();
        move_storage_from_pallet(prefix, OLD_PREFIX.as_bytes(), pallet_name.as_bytes());
        info!(
            target: LOG_TARGET,
            "Migrated underperformed_validator_session_count"
        );

        let prefix = crate::Banned::<T>::storage_prefix();
        move_storage_from_pallet(prefix, OLD_PREFIX.as_bytes(), pallet_name.as_bytes());
        info!(target: LOG_TARGET, "Migrated Banned");

        StorageVersion::new(1).put::<Pallet<T>>();

        <T as frame_system::Config>::BlockWeights::get().max_block
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
        ensure_storage_version::<Pallet<T>>(0)?;

        let pallet_name = Pallet::<T>::name();

        let pallet_prefix = twox_128(pallet_name.as_bytes());
        let storage_version_key = twox_128(STORAGE_VERSION_STORAGE_KEY_POSTFIX);

        let mut pallet_prefix_iter = frame_support::storage::KeyPrefixIterator::new(
            pallet_prefix.to_vec(),
            pallet_prefix.to_vec(),
            |key| Ok(key.to_vec()),
        );

        // Ensure nothing except the storage_version_key is stored in the new prefix.
        ensure!(
            pallet_prefix_iter.all(|key| key == storage_version_key),
            "Only storage version should be stored in the pallet"
        );

        Ok(MigrationStateCheck {
            session_validator_block_count: elections::SessionValidatorBlockCount::<T>::iter()
                .count() as u32,
            validator_era_total_reward: elections::ValidatorEraTotalReward::<T>::get(),
            ban_config: elections::BanConfig::get().expect("It still should be here"),
            underperformed_validator_session_count:
                elections::UnderperformedValidatorSessionCount::<T>::iter().count() as u32,
            banned: elections::Banned::<T>::iter().count() as u32,
        }
        .encode())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(state: Vec<u8>) -> Result<(), &'static str> {
        ensure_storage_version::<Pallet<T>>(0)?;

        // Ensure storages has been moved to new prefix.
        let state = <MigrationStateCheck<T::AccountId>>::decode(&mut &*state)
            .map_err(|_| "Failed to decode")?;

        ensure!(
            state
                == MigrationStateCheck {
                    session_validator_block_count: crate::SessionValidatorBlockCount::<T>::iter()
                        .count() as u32,
                    validator_era_total_reward: crate::ValidatorEraTotalReward::<T>::get(),
                    ban_config: crate::BanConfig::<T>::get(),
                    underperformed_validator_session_count:
                        crate::UnderperformedValidatorSessionCount::<T>::iter().count() as u32,
                    banned: crate::Banned::<T>::iter().count() as u32,
                },
            "Moved storages are not the same"
        );

        Ok(())
    }
}
