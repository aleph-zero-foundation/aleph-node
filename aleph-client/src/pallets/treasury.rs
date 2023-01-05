use frame_support::PalletId;
use primitives::{Balance, MILLISECS_PER_BLOCK};
use sp_runtime::traits::AccountIdConversion;
use subxt::ext::sp_runtime::MultiAddress;

use crate::{
    api,
    connections::AsConnection,
    pallet_treasury::pallet::Call::{approve_proposal, reject_proposal},
    pallets::{elections::ElectionsApi, staking::StakingApi},
    AccountId, BlockHash,
    Call::Treasury,
    ConnectionApi, RootConnection, SignedConnectionApi, SudoCall, TxStatus,
};

/// Pallet treasury read-only api.
#[async_trait::async_trait]
pub trait TreasuryApi {
    /// Returns an unique account id for all treasury transfers.
    async fn treasury_account(&self) -> AccountId;

    /// Returns storage `proposals_count`.
    /// * `at` - an optional block hash to query state from
    async fn proposals_count(&self, at: Option<BlockHash>) -> Option<u32>;

    /// Returns storage `approvals`.
    /// * `at` - an optional block hash to query state from
    async fn approvals(&self, at: Option<BlockHash>) -> Vec<u32>;
}

/// Pallet treasury api.
#[async_trait::async_trait]
pub trait TreasuryUserApi {
    /// API for [`propose_spend`](https://paritytech.github.io/substrate/master/pallet_treasury/pallet/struct.Pallet.html#method.propose_spend) call.
    async fn propose_spend(
        &self,
        amount: Balance,
        beneficiary: AccountId,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;

    /// API for [`approve_proposal`](https://paritytech.github.io/substrate/master/pallet_treasury/pallet/struct.Pallet.html#method.approve_proposal) call.
    async fn approve(&self, proposal_id: u32, status: TxStatus) -> anyhow::Result<BlockHash>;

    /// API for [`reject_proposal`](https://paritytech.github.io/substrate/master/pallet_treasury/pallet/struct.Pallet.html#method.reject_proposal) call.
    async fn reject(&self, proposal_id: u32, status: TxStatus) -> anyhow::Result<BlockHash>;
}

/// Pallet treasury funcionality that is not directly related to any pallet call.
#[async_trait::async_trait]
pub trait TreasureApiExt {
    /// When `staking.payout_stakers` is done, what amount of AZERO is transferred to.
    /// the treasury
    async fn possible_treasury_payout(&self) -> anyhow::Result<Balance>;
}

/// Pallet treasury api issued by the sudo account.
#[async_trait::async_trait]
pub trait TreasurySudoApi {
    /// API for [`approve_proposal`](https://paritytech.github.io/substrate/master/pallet_treasury/pallet/struct.Pallet.html#method.approve_proposal) call.
    /// wrapped  in [`sudo_unchecked_weight`](https://paritytech.github.io/substrate/master/pallet_sudo/pallet/struct.Pallet.html#method.sudo_unchecked_weight)
    async fn approve(&self, proposal_id: u32, status: TxStatus) -> anyhow::Result<BlockHash>;

    /// API for [`reject_proposal`](https://paritytech.github.io/substrate/master/pallet_treasury/pallet/struct.Pallet.html#method.reject_proposal) call.
    /// wrapped [`sudo_unchecked_weight`](https://paritytech.github.io/substrate/master/pallet_sudo/pallet/struct.Pallet.html#method.sudo_unchecked_weight)
    async fn reject(&self, proposal_id: u32, status: TxStatus) -> anyhow::Result<BlockHash>;
}

#[async_trait::async_trait]
impl<C: ConnectionApi> TreasuryApi for C {
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
impl<S: SignedConnectionApi> TreasuryUserApi for S {
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
impl<C: AsConnection + Sync> TreasureApiExt for C {
    async fn possible_treasury_payout(&self) -> anyhow::Result<Balance> {
        let session_period = self.get_session_period().await?;
        let sessions_per_era = self.get_session_per_era().await?;
        let millisecs_per_era =
            MILLISECS_PER_BLOCK * session_period as u64 * sessions_per_era as u64;
        Ok(primitives::staking::era_payout(millisecs_per_era).1)
    }
}
