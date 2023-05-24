use crate::{
    api,
    sp_runtime::{traits::One, FixedU128},
    BlockHash, ConnectionApi,
};

/// Transaction payment pallet API.
#[async_trait::async_trait]
pub trait TransactionPaymentApi {
    /// API for [`next_fee_multiplier`](https://paritytech.github.io/substrate/master/pallet_transaction_payment/pallet/struct.Pallet.html#method.next_fee_multiplier) call.
    async fn get_next_fee_multiplier(&self, at: Option<BlockHash>) -> FixedU128;
}

#[async_trait::async_trait]
impl<C: ConnectionApi> TransactionPaymentApi for C {
    async fn get_next_fee_multiplier(&self, at: Option<BlockHash>) -> FixedU128 {
        let addrs = api::storage().transaction_payment().next_fee_multiplier();

        self.get_storage_entry_maybe(&addrs, at)
            .await
            .map_or(FixedU128::one(), |f| FixedU128::from_inner(f.0))
    }
}
