use std::{fs, path::PathBuf};

use aleph_client::{pallets::vk_storage::VkStorageUserApi, SignedConnection, TxStatus};
use anyhow::Result;

fn read_bytes(file: &PathBuf) -> Result<Vec<u8>> {
    fs::read(file).map_err(|e| e.into())
}

/// Calls `pallet_vk_storage::store_key`.
pub async fn store_key(connection: SignedConnection, vk_file: PathBuf) -> Result<()> {
    let vk = read_bytes(&vk_file)?;
    connection
        .store_key(vk, TxStatus::Finalized)
        .await
        .map(|_| ())
}
