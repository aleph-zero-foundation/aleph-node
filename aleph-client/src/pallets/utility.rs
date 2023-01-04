use crate::{api, BlockHash, Call, SignedConnectionApi, TxStatus};

#[async_trait::async_trait]
pub trait UtilityApi {
    async fn batch_call(&self, calls: Vec<Call>, status: TxStatus) -> anyhow::Result<BlockHash>;
}

#[async_trait::async_trait]
impl<S: SignedConnectionApi> UtilityApi for S {
    async fn batch_call(&self, calls: Vec<Call>, status: TxStatus) -> anyhow::Result<BlockHash> {
        let tx = api::tx().utility().batch(calls);

        self.send_tx(tx, status).await
    }
}
