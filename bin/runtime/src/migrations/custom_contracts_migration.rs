use codec::{Decode, DecodeAll, Encode, Error};
use frame_support::{
    codec, log,
    pallet_prelude::*,
    storage_alias,
    traits::{Currency, Get, OnRuntimeUpgrade},
    weights::Weight,
    Twox64Concat,
};
use pallet_contracts::{Config, Pallet};
use sp_runtime::traits::Saturating;
use sp_std::{marker::PhantomData, prelude::*};

use crate::sp_api_hidden_includes_construct_runtime::hidden_include::StoragePrefixedMap;

type CodeHash<T> = <T as frame_system::Config>::Hash;
type TrieId = BoundedVec<u8, ConstU32<128>>;

type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

const TARGET: &str = "runtime::custom_contract_migration";

/// Performs all necessary migrations based on `StorageVersion`.
pub struct CustomMigrateToV8<T: Config>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for CustomMigrateToV8<T> {
    fn on_runtime_upgrade() -> Weight {
        let version = StorageVersion::get::<Pallet<T>>();
        let mut weight = Weight::zero();
        log::info!(
            target: TARGET,
            "On-chain version of pallet contracts is {:?}",
            version
        );
        if version < 8 {
            let (used_weight, finished) = v8::migrate::<T>();
            weight = weight.saturating_add(used_weight);
            if finished {
                StorageVersion::new(8).put::<Pallet<T>>();
                log::info!(
                    target: TARGET,
                    "All contracts migrated. Writing new StorageVersion.",
                );
            }
        }

        weight
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
        let version = StorageVersion::get::<Pallet<T>>();
        log::warn!(
            target: TARGET,
            "Pre-upgrade in custom contracts migration. {:?}",
            version
        );
        if version < 8 {
            v8::pre_upgrade::<T>()?;
        }

        Ok(version.encode())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(state: Vec<u8>) -> Result<(), &'static str> {
        let version_new = StorageVersion::get::<Pallet<T>>();
        let version_old: StorageVersion =
            Decode::decode(&mut state.as_ref()).map_err(|_| "Cannot decode version")?;
        log::warn!(
            target: TARGET,
            "Post-upgrade in custom contracts migration from version {:?} to {:?}",
            version_old,
            version_new
        );
        if version_old <= StorageVersion::new(7) && version_new == StorageVersion::new(8) {
            v8::post_upgrade::<T>()?;
        }
        Ok(())
    }
}

/// Update `ContractInfo` with new fields that track storage deposits.
mod v8 {
    use frame_support::storage::unhashed;
    use sp_io::default_child_storage as child;

    use super::*;

    #[derive(Encode, Decode)]
    struct RawContractInfo<CodeHash, Balance> {
        pub trie_id: TrieId,
        pub code_hash: CodeHash,
        pub storage_deposit: Balance,
    }

    type OldContractInfo<T> = RawContractInfo<CodeHash<T>, BalanceOf<T>>;

    #[derive(Encode, Decode)]
    struct ContractInfo<T: Config> {
        trie_id: TrieId,
        code_hash: CodeHash<T>,
        storage_bytes: u32,
        storage_items: u32,
        storage_byte_deposit: BalanceOf<T>,
        storage_item_deposit: BalanceOf<T>,
        storage_base_deposit: BalanceOf<T>,
    }

    const OLD_INFO_ENCODING_LEN: usize = 33 + 32 + 16;
    const NEW_INFO_ENCODING_LEN: usize = 33 + 32 + 4 + 4 + 16 + 16 + 16;
    const MAX_NUM_ITEMS_PROCESSED_PER_MIGRATION: u32 = 140000;

    enum EitherContractInto<T: Config> {
        Old(OldContractInfo<T>),
        New(ContractInfo<T>),
    }

    #[storage_alias]
    type ContractInfoOf<T: Config, V> =
        StorageMap<Pallet<T>, Twox64Concat, <T as frame_system::Config>::AccountId, V>;

    fn decode_to_either_contract_info<T: Config>(
        input: &mut &[u8],
    ) -> Result<EitherContractInto<T>, Error> {
        let len = input.len();
        match len {
            OLD_INFO_ENCODING_LEN => {
                if let Ok(c_info) = OldContractInfo::<T>::decode_all(input) {
                    Ok(EitherContractInto::Old(c_info))
                } else {
                    Err(Error::from("Failed to decode ContractInfo to Old type"))
                }
            }
            NEW_INFO_ENCODING_LEN => {
                if let Ok(c_info) = ContractInfo::<T>::decode_all(input) {
                    Ok(EitherContractInto::New(c_info))
                } else {
                    Err(Error::from("Failed to decode ContractInfo to New type"))
                }
            }
            _ => Err(Error::from("Wrong size of ContractInfo")),
        }
    }

    // Returns the used weight and whether this migration is the final one.
    pub fn migrate<T: Config>() -> (Weight, bool) {
        log::info!(
            target: TARGET,
            "Running v7->v8 migration of pallet contracts"
        );
        let mut weight = Weight::zero();
        let mut total_bytes: u32 = 0;
        let mut total_items: u32 = 0;
        let mut num_old_contracts_migrated: u32 = 0;
        let mut num_old_contracts_skipped: u32 = 0;
        let mut num_new_contracts: u32 = 0;
        let mut num_contracts_erred: u32 = 0;

        // Note that we use V = ContractInfo<T> and not V = OldContractInfo<T>, but that does not matter for the prefix.
        let prefix = ContractInfoOf::<T, ContractInfo<T>>::final_prefix();
        let mut previous_key = prefix.clone().to_vec();
        // Iterate over all elements of the ContractInfoOf map
        while let Some(next) =
            sp_io::storage::next_key(&previous_key).filter(|n| n.starts_with(&prefix))
        {
            previous_key = next;
            let value = sp_io::storage::get(&previous_key)
                .expect("The loop conditions checks for existence of the key; qed");
            match decode_to_either_contract_info::<T>(&mut &value[..]) {
                Ok(EitherContractInto::Old(old)) => {
                    if total_items >= MAX_NUM_ITEMS_PROCESSED_PER_MIGRATION {
                        num_old_contracts_skipped.saturating_accrue(1);
                        continue;
                    }
                    let mut storage_bytes = 0u32;
                    let mut storage_items = 0u32;
                    let mut child_key = Vec::new();
                    while let Some(next_child_key) = child::next_key(&old.trie_id, &child_key) {
                        child_key = next_child_key;
                        let mut val_out = [];
                        let len = child::read(&old.trie_id, &child_key, &mut val_out, 0)
                            .expect("The loop conditions checks for existence of the key; qed");
                        storage_bytes.saturating_accrue(len);
                        storage_items.saturating_accrue(1);
                    }

                    total_bytes.saturating_accrue(storage_bytes);
                    total_items.saturating_accrue(storage_items);

                    let storage_byte_deposit =
                        T::DepositPerByte::get().saturating_mul(storage_bytes.into());
                    let storage_item_deposit =
                        T::DepositPerItem::get().saturating_mul(storage_items.into());
                    let storage_base_deposit = old
                        .storage_deposit
                        .saturating_sub(storage_byte_deposit)
                        .saturating_sub(storage_item_deposit);

                    let new_contract_info = ContractInfo {
                        trie_id: old.trie_id,
                        code_hash: old.code_hash,
                        storage_bytes,
                        storage_items,
                        storage_byte_deposit,
                        storage_item_deposit,
                        storage_base_deposit,
                    };
                    unhashed::put::<ContractInfo<T>>(&previous_key, &new_contract_info);

                    weight = weight.saturating_add(
                        T::DbWeight::get().reads_writes(u64::from(storage_items) + 1, 1),
                    );

                    num_old_contracts_migrated.saturating_accrue(1);
                }
                Ok(EitherContractInto::New(_)) => {
                    weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 0));
                    num_new_contracts.saturating_accrue(1);
                }
                Err(e) => {
                    log::info!(
                        target: TARGET,
                        "Error {:?} when decoding ContractInfo at key {:?}",
                        previous_key,
                        e
                    );
                    num_contracts_erred.saturating_accrue(1);
                }
            }
        }
        log::info!(
            target: TARGET,
            "Migration ended. old migrated {}, old skipped {}, new skipped {}, err {}. Stats: total_bytes {:?}, total_items {:?}",
			num_old_contracts_migrated,
			num_old_contracts_skipped,
			num_new_contracts,
            num_contracts_erred,
            total_bytes,
            total_items
        );
        (weight, num_old_contracts_skipped == 0)
    }

    #[cfg(feature = "try-runtime")]
    pub fn pre_upgrade<T: Config>() -> Result<(), &'static str> {
        use frame_support::traits::ReservableCurrency;
        let mut cnt: u32 = 0;
        for (key, value) in ContractInfoOf::<T, OldContractInfo<T>>::iter() {
            cnt += 1;
            let reserved = T::Currency::reserved_balance(&key);
            if reserved < value.storage_deposit {
                log::warn!(
                    target: TARGET,
                    "Issue in pre-upgrade at num {}  {:?} {:?} {:?}",
                    cnt,
                    key.encode(),
                    value.storage_deposit,
                    reserved
                );
            }
        }
        Ok(())
    }

    #[cfg(feature = "try-runtime")]
    pub fn post_upgrade<T: Config>() -> Result<(), &'static str> {
        use frame_support::traits::ReservableCurrency;
        for (key, value) in ContractInfoOf::<T, ContractInfo<T>>::iter() {
            let reserved = T::Currency::reserved_balance(&key);
            let stored = value
                .storage_base_deposit
                .saturating_add(value.storage_byte_deposit)
                .saturating_add(value.storage_item_deposit);
            if reserved < stored {
                log::warn!(
                    target: TARGET,
                    "Issue in post-upgrade at num {:?} {:?} {:?}",
                    key.encode(),
                    stored,
                    reserved
                );
            }
            let mut storage_bytes = 0u32;
            let mut storage_items = 0u32;
            let mut key = Vec::new();
            while let Some(next) = child::next_key(&value.trie_id, &key) {
                key = next;
                let mut val_out = [];
                let len = child::read(&value.trie_id, &key, &mut val_out, 0)
                    .expect("The loop conditions checks for existence of the key; qed");
                storage_bytes.saturating_accrue(len);
                storage_items.saturating_accrue(1);
            }
            ensure!(
                storage_bytes == value.storage_bytes,
                "Storage bytes do not match.",
            );
            ensure!(
                storage_items == value.storage_items,
                "Storage items do not match.",
            );
        }
        Ok(())
    }
}
