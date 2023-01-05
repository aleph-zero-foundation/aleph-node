use subxt::ext::sp_runtime::MultiAddress;

use crate::{
    api, pallet_vesting::vesting_info::VestingInfo, AccountId, BlockHash, ConnectionApi,
    SignedConnectionApi, TxStatus,
};

/// Read only pallet vesting API.
#[async_trait::async_trait]
pub trait VestingApi {
    /// Returns [`VestingInfo`] of the given account.
    /// * `who` - an account id
    /// * `at` - optional hash of a block to query state from
    async fn get_vesting(
        &self,
        who: AccountId,
        at: Option<BlockHash>,
    ) -> Vec<VestingInfo<u128, u32>>;
}

/// Pallet vesting api.
#[async_trait::async_trait]
pub trait VestingUserApi {
    /// API for [`vest`](https://paritytech.github.io/substrate/master/pallet_vesting/pallet/enum.Call.html#variant.vest) call.
    async fn vest(&self, status: TxStatus) -> anyhow::Result<BlockHash>;

    /// API for [`vest_other`](https://paritytech.github.io/substrate/master/pallet_vesting/pallet/enum.Call.html#variant.vest_other) call.
    async fn vest_other(&self, status: TxStatus, other: AccountId) -> anyhow::Result<BlockHash>;

    /// API for [`vested_transfer`](https://paritytech.github.io/substrate/master/pallet_vesting/pallet/enum.Call.html#variant.vested_transfer) call.
    async fn vested_transfer(
        &self,
        receiver: AccountId,
        schedule: VestingInfo<u128, u32>,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;

    /// API for [`merge_schedules`](https://paritytech.github.io/substrate/master/pallet_vesting/pallet/enum.Call.html#variant.merge_schedules) call.
    async fn merge_schedules(
        &self,
        idx1: u32,
        idx2: u32,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;
}

#[async_trait::async_trait]
impl<C: ConnectionApi> VestingApi for C {
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
impl<S: SignedConnectionApi> VestingUserApi for S {
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
