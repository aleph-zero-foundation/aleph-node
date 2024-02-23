use anyhow::Result;

use crate::{
    api, sp_core::H256, BlockHash, ConnectionApi, SignedConnection, SignedConnectionApi, TxInfo,
    TxStatus,
};

/// Read only pallet vk storage API.
#[async_trait::async_trait]
pub trait VkStorageApi {
    /// Get verification key from pallet's storage.
    async fn get_verification_key(&self, key_hash: H256, at: Option<BlockHash>) -> Vec<u8>;
}

/// Pallet vk storage API.
#[async_trait::async_trait]
pub trait VkStorageUserApi {
    /// Store a verifying key in pallet's storage.
    async fn store_key(&self, key: Vec<u8>, status: TxStatus) -> Result<TxInfo>;
}

#[async_trait::async_trait]
impl<C: ConnectionApi> VkStorageApi for C {
    async fn get_verification_key(&self, key_hash: H256, at: Option<BlockHash>) -> Vec<u8> {
        let addrs = api::storage().vk_storage().verification_keys(key_hash);
        self.get_storage_entry(&addrs, at).await.0
    }
}

#[async_trait::async_trait]
impl VkStorageUserApi for SignedConnection {
    async fn store_key(&self, key: Vec<u8>, status: TxStatus) -> Result<TxInfo> {
        let tx = api::tx().vk_storage().store_key(key);
        self.send_tx(tx, status).await
    }
}
