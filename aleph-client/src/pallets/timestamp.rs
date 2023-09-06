use crate::{api, BlockHash, ConnectionApi};

/// Timestamp payment pallet API.
#[async_trait::async_trait]
pub trait TimestampApi {
    /// API for [`get`](https://paritytech.github.io/substrate/master/pallet_timestamp/pallet/struct.Pallet.html#method.get) call.
    async fn get_timestamp(&self, at: Option<BlockHash>) -> Option<u64>;
}

#[async_trait::async_trait]
impl<C: ConnectionApi> TimestampApi for C {
    async fn get_timestamp(&self, at: Option<BlockHash>) -> Option<u64> {
        let addrs = api::storage().timestamp().now();
        self.get_storage_entry_maybe(&addrs, at).await
    }
}
