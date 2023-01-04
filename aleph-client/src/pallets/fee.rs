use crate::{api, BlockHash, ConnectionApi};

pub type FeeMultiplier = u128;

#[async_trait::async_trait]
pub trait TransactionPaymentApi {
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
