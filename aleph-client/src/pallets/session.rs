use crate::{
    api, api::runtime_types::aleph_runtime::SessionKeys, connections::TxInfo, AccountId, BlockHash,
    ConnectionApi, SessionIndex, SignedConnectionApi, TxStatus,
};

/// Pallet session read-only api.
#[async_trait::async_trait]
pub trait SessionApi {
    /// API for [`next_keys`](https://paritytech.github.io/substrate/master/pallet_session/pallet/type.NextKeys.html) call.
    async fn get_next_session_keys(
        &self,
        account: AccountId,
        at: Option<BlockHash>,
    ) -> Option<SessionKeys>;

    /// API for [`current_index`](https://paritytech.github.io/substrate/master/pallet_session/pallet/struct.Pallet.html#method.current_index) call.
    async fn get_session(&self, at: Option<BlockHash>) -> SessionIndex;

    /// API for [`validators`](https://paritytech.github.io/substrate/master/pallet_session/pallet/struct.Pallet.html#method.validators) call.
    async fn get_validators(&self, at: Option<BlockHash>) -> Vec<AccountId>;
}

/// any object that implements pallet session api
#[async_trait::async_trait]
pub trait SessionUserApi {
    /// API for [`set_keys`](https://paritytech.github.io/substrate/master/pallet_session/pallet/struct.Pallet.html#method.set_keys) call.
    async fn set_keys(&self, new_keys: SessionKeys, status: TxStatus) -> anyhow::Result<TxInfo>;
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
    async fn set_keys(&self, new_keys: SessionKeys, status: TxStatus) -> anyhow::Result<TxInfo> {
        let tx = api::tx().session().set_keys(new_keys, vec![]);

        self.send_tx(tx, status).await
    }
}
