use primitives::Balance;
use subxt::ext::sp_runtime::Perbill as SPerbill;

use crate::{
    api,
    frame_system::pallet::Call::{fill_block, set_code},
    sp_arithmetic::per_things::Perbill,
    AccountId, BlockHash,
    Call::System,
    Connection, RootConnection, SudoCall, TxStatus,
};

#[async_trait::async_trait]
pub trait SystemApi {
    async fn get_free_balance(&self, account: AccountId, at: Option<BlockHash>) -> Balance;
}

#[async_trait::async_trait]
pub trait SystemSudoApi {
    async fn set_code(&self, code: Vec<u8>, status: TxStatus) -> anyhow::Result<BlockHash>;
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
impl SystemApi for Connection {
    async fn get_free_balance(&self, account: AccountId, at: Option<BlockHash>) -> Balance {
        let addrs = api::storage().system().account(&account);

        match self.get_storage_entry_maybe(&addrs, at).await {
            None => 0,
            Some(account) => account.data.free,
        }
    }
}
