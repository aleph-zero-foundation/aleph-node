use crate::{api, BlockHash, ConnectionApi};

/// An alias for a fee multiplier.
pub type FeeMultiplier = u128;

/// Transaction payment pallet API.
#[async_trait::async_trait]
pub trait TransactionPaymentApi {
    /// API for [`next_fee_multiplier`](https://paritytech.github.io/substrate/master/pallet_transaction_payment/pallet/struct.Pallet.html#method.next_fee_multiplier) call.
    async fn get_next_fee_multiplier(&self, at: Option<BlockHash>) -> FeeMultiplier;
}

#[async_trait::async_trait]
impl<C: ConnectionApi> TransactionPaymentApi for C {
    async fn get_next_fee_multiplier(&self, at: Option<BlockHash>) -> FeeMultiplier {
        let addrs = api::storage().transaction_payment().next_fee_multiplier();

        match self.get_storage_entry_maybe(&addrs, at).await {
            Some(fm) => fm.0,
            None => 1,
        }
    }
}
