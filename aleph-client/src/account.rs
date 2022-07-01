use codec::Decode;
use pallet_balances::BalanceLock;
use sp_core::{crypto::AccountId32, storage::StorageKey};
use substrate_api_client::{AccountId, Balance};

use crate::{state_query_storage_at, AnyConnection};

pub fn get_free_balance<C: AnyConnection>(connection: &C, account: &AccountId) -> Balance {
    match connection
        .as_connection()
        .get_account_data(account)
        .expect("Should be able to access account data")
    {
        Some(account_data) => account_data.free,
        // Account may have not been initialized yet or liquidated due to the lack of funds.
        None => 0,
    }
}

pub fn locks<C: AnyConnection>(
    connection: &C,
    accounts: &[AccountId],
) -> Vec<Vec<BalanceLock<Balance>>> {
    let storage_keys = create_storage_keys_from_accounts(connection, accounts);
    get_locked_balances_from_storage(connection, storage_keys)
}

fn create_storage_keys_from_accounts<C: AnyConnection>(
    connection: &C,
    accounts: &[AccountId32],
) -> Vec<StorageKey> {
    accounts
        .iter()
        .map(|account| {
            connection
                .as_connection()
                .metadata
                .storage_map_key("Balances", "Locks", account)
                .unwrap_or_else(|_| panic!("Cannot create storage key for account {}!", account))
        })
        .collect()
}

fn get_locked_balances_from_storage<C: AnyConnection>(
    connection: &C,
    storage_keys: Vec<StorageKey>,
) -> Vec<Vec<BalanceLock<Balance>>> {
    match state_query_storage_at(connection, storage_keys) {
        Ok(storage_entries) => storage_entries
            .into_iter()
            .map(|storage_entry| {
                let entry_bytes = storage_entry.expect("Storage entry is null!").0;
                Decode::decode(&mut entry_bytes.as_slice())
                    .expect("Failed to decode locked balances!")
            })
            .collect(),
        Err(err) => {
            panic!("Failed to query storage, details: {}", &err[..]);
        }
    }
}
