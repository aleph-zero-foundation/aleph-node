use crate::{
    api, pallet_feature_control::Feature, BlockHash, ConnectionApi, RootConnection,
    SignedConnectionApi, TxInfo, TxStatus,
};

/// Read only pallet feature control API.
#[async_trait::async_trait]
pub trait FeatureControlApi {
    /// Check if a feature is active.
    async fn is_feature_active(&self, feature: Feature, at: Option<BlockHash>) -> bool;
}

/// Pallet feature control API that requires sudo.
#[async_trait::async_trait]
pub trait FeatureControlSudoApi {
    /// Enable a feature.
    async fn enable_feature(&self, feature: Feature, status: TxStatus) -> anyhow::Result<TxInfo>;
    /// Disable a feature.
    async fn disable_feature(&self, feature: Feature, status: TxStatus) -> anyhow::Result<TxInfo>;
}

#[async_trait::async_trait]
impl<C: ConnectionApi> FeatureControlApi for C {
    async fn is_feature_active(&self, feature: Feature, at: Option<BlockHash>) -> bool {
        let addrs = api::storage().feature_control().active_features(feature);
        self.get_storage_entry_maybe(&addrs, at).await.is_some()
    }
}

#[async_trait::async_trait]
impl FeatureControlSudoApi for RootConnection {
    async fn enable_feature(&self, feature: Feature, status: TxStatus) -> anyhow::Result<TxInfo> {
        let tx = api::tx().feature_control().enable(feature);
        self.send_tx(tx, status).await
    }

    async fn disable_feature(&self, feature: Feature, status: TxStatus) -> anyhow::Result<TxInfo> {
        let tx = api::tx().feature_control().disable(feature);
        self.send_tx(tx, status).await
    }
}
