use anyhow::Result;

use crate::{api, SignedConnection, SignedConnectionApi, TxInfo, TxStatus};

/// Pallet vk storage API.
#[async_trait::async_trait]
pub trait VkStorageUserApi {
    /// Store verifying key in pallet's storage.
    async fn store_key(&self, key: Vec<u8>, status: TxStatus) -> Result<TxInfo>;
}

#[async_trait::async_trait]
impl VkStorageUserApi for SignedConnection {
    async fn store_key(&self, key: Vec<u8>, status: TxStatus) -> Result<TxInfo> {
        let tx = api::tx().vk_storage().store_key(key);
        self.send_tx(tx, status).await
    }
}
