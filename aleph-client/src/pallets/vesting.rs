use subxt::ext::sp_runtime::MultiAddress;

use crate::{
    api, pallet_vesting::vesting_info::VestingInfo, AccountId, BlockHash, Connection,
    SignedConnection, TxStatus,
};

#[async_trait::async_trait]
pub trait VestingApi {
    async fn get_vesting(
        &self,
        who: AccountId,
        at: Option<BlockHash>,
    ) -> Vec<VestingInfo<u128, u32>>;
}

#[async_trait::async_trait]
pub trait VestingUserApi {
    async fn vest(&self, status: TxStatus) -> anyhow::Result<BlockHash>;
    async fn vest_other(&self, status: TxStatus, other: AccountId) -> anyhow::Result<BlockHash>;
    async fn vested_transfer(
        &self,
        receiver: AccountId,
        schedule: VestingInfo<u128, u32>,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;
    async fn merge_schedules(
        &self,
        idx1: u32,
        idx2: u32,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;
}

#[async_trait::async_trait]
impl VestingApi for Connection {
    async fn get_vesting(
        &self,
        who: AccountId,
        at: Option<BlockHash>,
    ) -> Vec<VestingInfo<u128, u32>> {
        let addrs = api::storage().vesting().vesting(who);

        self.get_storage_entry(&addrs, at).await.0
    }
}

#[async_trait::async_trait]
impl VestingUserApi for SignedConnection {
    async fn vest(&self, status: TxStatus) -> anyhow::Result<BlockHash> {
        let tx = api::tx().vesting().vest();

        self.send_tx(tx, status).await
    }

    async fn vest_other(&self, status: TxStatus, other: AccountId) -> anyhow::Result<BlockHash> {
        let tx = api::tx().vesting().vest_other(MultiAddress::Id(other));

        self.send_tx(tx, status).await
    }

    async fn vested_transfer(
        &self,
        receiver: AccountId,
        schedule: VestingInfo<u128, u32>,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash> {
        let tx = api::tx()
            .vesting()
            .vested_transfer(MultiAddress::Id(receiver), schedule);

        self.send_tx(tx, status).await
    }

    async fn merge_schedules(
        &self,
        idx1: u32,
        idx2: u32,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash> {
        let tx = api::tx().vesting().merge_schedules(idx1, idx2);

        self.send_tx(tx, status).await
    }
}
