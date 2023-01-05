use primitives::Balance;
use subxt::ext::sp_runtime::Perbill as SPerbill;

use crate::{
    api,
    frame_system::pallet::Call::{fill_block, set_code},
    sp_arithmetic::per_things::Perbill,
    AccountId, BlockHash,
    Call::System,
    ConnectionApi, RootConnection, SudoCall, TxStatus,
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
    async fn set_code(&self, code: Vec<u8>, status: TxStatus) -> anyhow::Result<BlockHash>;

    /// A dispatch that will fill the block weight up to the given ratio.
    /// * `target_ratio_percent` - ratio to fill block
    /// `status` - a [`TxStatus`] to wait for
    async fn fill_block(
        &self,
        target_ratio_percent: u8,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;
}

#[async_trait::async_trait]
impl SystemSudoApi for RootConnection {
    async fn set_code(&self, code: Vec<u8>, status: TxStatus) -> anyhow::Result<BlockHash> {
        let call = System(set_code { code });

        self.sudo_unchecked(call, status).await
    }

    async fn fill_block(
        &self,
        target_ratio_percent: u8,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash> {
        let call = System(fill_block {
            ratio: Perbill(SPerbill::from_percent(target_ratio_percent as u32).deconstruct()),
        });

        self.sudo(call, status).await
    }
}

#[async_trait::async_trait]
impl<C: ConnectionApi> SystemApi for C {
    async fn get_free_balance(&self, account: AccountId, at: Option<BlockHash>) -> Balance {
        let addrs = api::storage().system().account(&account);

        match self.get_storage_entry_maybe(&addrs, at).await {
            None => 0,
            Some(account) => account.data.free,
        }
    }
}
