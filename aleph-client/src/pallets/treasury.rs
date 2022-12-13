use frame_support::PalletId;
use primitives::{Balance, MILLISECS_PER_BLOCK};
use sp_runtime::traits::AccountIdConversion;
use subxt::ext::sp_runtime::MultiAddress;

use crate::{
    api,
    pallet_treasury::pallet::Call::{approve_proposal, reject_proposal},
    pallets::{elections::ElectionsApi, staking::StakingApi},
    AccountId, BlockHash,
    Call::Treasury,
    Connection, RootConnection, SignedConnection, SudoCall, TxStatus,
};

#[async_trait::async_trait]
pub trait TreasuryApi {
    async fn treasury_account(&self) -> AccountId;
    async fn proposals_count(&self, at: Option<BlockHash>) -> Option<u32>;
    async fn approvals(&self, at: Option<BlockHash>) -> Vec<u32>;
}

#[async_trait::async_trait]
pub trait TreasuryUserApi {
    async fn propose_spend(
        &self,
        amount: Balance,
        beneficiary: AccountId,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;
    async fn approve(&self, proposal_id: u32, status: TxStatus) -> anyhow::Result<BlockHash>;
    async fn reject(&self, proposal_id: u32, status: TxStatus) -> anyhow::Result<BlockHash>;
}

#[async_trait::async_trait]
pub trait TreasureApiExt {
    async fn possible_treasury_payout(&self) -> Balance;
}

#[async_trait::async_trait]
pub trait TreasurySudoApi {
    async fn approve(&self, proposal_id: u32, status: TxStatus) -> anyhow::Result<BlockHash>;
    async fn reject(&self, proposal_id: u32, status: TxStatus) -> anyhow::Result<BlockHash>;
}

#[async_trait::async_trait]
impl TreasuryApi for Connection {
    async fn treasury_account(&self) -> AccountId {
        PalletId(*b"a0/trsry").into_account_truncating()
    }

    async fn proposals_count(&self, at: Option<BlockHash>) -> Option<u32> {
        let addrs = api::storage().treasury().proposal_count();

        self.get_storage_entry_maybe(&addrs, at).await
    }

    async fn approvals(&self, at: Option<BlockHash>) -> Vec<u32> {
        let addrs = api::storage().treasury().approvals();

        self.get_storage_entry(&addrs, at).await.0
    }
}

#[async_trait::async_trait]
impl TreasuryUserApi for SignedConnection {
    async fn propose_spend(
        &self,
        amount: Balance,
        beneficiary: AccountId,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash> {
        let tx = api::tx()
            .treasury()
            .propose_spend(amount, MultiAddress::Id(beneficiary));

        self.send_tx(tx, status).await
    }

    async fn approve(&self, proposal_id: u32, status: TxStatus) -> anyhow::Result<BlockHash> {
        let tx = api::tx().treasury().approve_proposal(proposal_id);

        self.send_tx(tx, status).await
    }

    async fn reject(&self, proposal_id: u32, status: TxStatus) -> anyhow::Result<BlockHash> {
        let tx = api::tx().treasury().reject_proposal(proposal_id);

        self.send_tx(tx, status).await
    }
}

#[async_trait::async_trait]
impl TreasurySudoApi for RootConnection {
    async fn approve(&self, proposal_id: u32, status: TxStatus) -> anyhow::Result<BlockHash> {
        let call = Treasury(approve_proposal { proposal_id });

        self.sudo_unchecked(call, status).await
    }

    async fn reject(&self, proposal_id: u32, status: TxStatus) -> anyhow::Result<BlockHash> {
        let call = Treasury(reject_proposal { proposal_id });

        self.sudo_unchecked(call, status).await
    }
}

#[async_trait::async_trait]
impl TreasureApiExt for Connection {
    async fn possible_treasury_payout(&self) -> Balance {
        let session_period = self.get_session_period().await;
        let sessions_per_era = self.get_session_per_era().await;

        let millisecs_per_era =
            MILLISECS_PER_BLOCK * session_period as u64 * sessions_per_era as u64;
        primitives::staking::era_payout(millisecs_per_era).1
    }
}
