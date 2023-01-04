use primitives::SessionIndex;

use crate::{
    api, api::runtime_types::aleph_runtime::SessionKeys, AccountId, BlockHash, ConnectionApi,
    SignedConnectionApi, TxStatus,
};

#[async_trait::async_trait]
pub trait SessionApi {
    async fn get_next_session_keys(
        &self,
        account: AccountId,
        at: Option<BlockHash>,
    ) -> Option<SessionKeys>;
    async fn get_session(&self, at: Option<BlockHash>) -> SessionIndex;
    async fn get_validators(&self, at: Option<BlockHash>) -> Vec<AccountId>;
}

#[async_trait::async_trait]
pub trait SessionUserApi {
    async fn set_keys(&self, new_keys: SessionKeys, status: TxStatus) -> anyhow::Result<BlockHash>;
}

#[async_trait::async_trait]
impl<C: ConnectionApi> SessionApi for C {
    async fn get_next_session_keys(
        &self,
        account: AccountId,
        at: Option<BlockHash>,
    ) -> Option<SessionKeys> {
        let addrs = api::storage().session().next_keys(account);

        self.get_storage_entry_maybe(&addrs, at).await
    }

    async fn get_session(&self, at: Option<BlockHash>) -> SessionIndex {
        let addrs = api::storage().session().current_index();

        self.get_storage_entry_maybe(&addrs, at)
            .await
            .unwrap_or_default()
    }

    async fn get_validators(&self, at: Option<BlockHash>) -> Vec<AccountId> {
        let addrs = api::storage().session().validators();

        self.get_storage_entry(&addrs, at).await
    }
}

#[async_trait::async_trait]
impl<S: SignedConnectionApi> SessionUserApi for S {
    async fn set_keys(&self, new_keys: SessionKeys, status: TxStatus) -> anyhow::Result<BlockHash> {
        let tx = api::tx().session().set_keys(new_keys, vec![]);

        self.send_tx(tx, status).await
    }
}
