use subxt::utils::Static;

use crate::{
    api, connections::TxInfo, frame_system::pallet::Call::set_code, AccountId, Balance, BlockHash,
    Call::System, ConnectionApi, RootConnection, SudoCall, TxStatus,
};

/// Pallet system read-only api.
#[async_trait::async_trait]
pub trait SystemApi {
    /// returns free balance of a given account
    /// * `account` - account id
    /// * `at` - optional hash of a block to query state from
    ///
    /// it uses [`system.account`](https://paritytech.github.io/substrate/master/frame_system/pallet/struct.Pallet.html#method.account) storage
    async fn get_free_balance(&self, account: AccountId, at: Option<BlockHash>) -> Balance;
}

/// Pallet system api.
#[async_trait::async_trait]
pub trait SystemSudoApi {
    /// API for [`set_code`](https://paritytech.github.io/substrate/master/frame_system/pallet/struct.Pallet.html#method.set_code) call.
    async fn set_code(&self, code: Vec<u8>, status: TxStatus) -> anyhow::Result<TxInfo>;
}

#[async_trait::async_trait]
impl SystemSudoApi for RootConnection {
    async fn set_code(&self, code: Vec<u8>, status: TxStatus) -> anyhow::Result<TxInfo> {
        let call = System(set_code { code });

        self.sudo_unchecked(call, status).await
    }
}

#[async_trait::async_trait]
impl<C: ConnectionApi> SystemApi for C {
    async fn get_free_balance(&self, account: AccountId, at: Option<BlockHash>) -> Balance {
        let addrs = api::storage().system().account(Static(account));

        match self.get_storage_entry_maybe(&addrs, at).await {
            None => 0,
            Some(account) => account.data.free,
        }
    }
}
