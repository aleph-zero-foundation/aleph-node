use crate::{api, BlockHash, Call, SignedConnection, TxStatus};

#[async_trait::async_trait]
pub trait UtilityApi {
    async fn batch_call(&self, calls: Vec<Call>, status: TxStatus) -> anyhow::Result<BlockHash>;
}

#[async_trait::async_trait]
impl UtilityApi for SignedConnection {
    async fn batch_call(&self, calls: Vec<Call>, status: TxStatus) -> anyhow::Result<BlockHash> {
        let tx = api::tx().utility().batch(calls);

        self.send_tx(tx, status).await
    }
}
