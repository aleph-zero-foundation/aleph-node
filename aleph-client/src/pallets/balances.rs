use primitives::Balance;
use subxt::{ext::sp_runtime::MultiAddress, tx::PolkadotExtrinsicParamsBuilder};

use crate::{
    aleph_zero::{self, api, api::runtime_types::pallet_balances::BalanceLock},
    pallet_balances::pallet::Call::transfer,
    pallets::utility::UtilityApi,
    AccountId, BlockHash,
    Call::Balances,
    ConnectionApi, SignedConnectionApi, TxStatus,
};

#[async_trait::async_trait]
pub trait BalanceApi {
    async fn locks_for_account(
        &self,
        account: AccountId,
        at: Option<BlockHash>,
    ) -> Vec<BalanceLock<Balance>>;
    async fn locks(
        &self,
        accounts: &[AccountId],
        at: Option<BlockHash>,
    ) -> Vec<Vec<BalanceLock<Balance>>>;
    async fn total_issuance(&self, at: Option<BlockHash>) -> Balance;
}

#[async_trait::async_trait]
pub trait BalanceUserApi {
    async fn transfer(
        &self,
        dest: AccountId,
        amount: Balance,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;
    async fn transfer_with_tip(
        &self,
        dest: AccountId,
        amount: Balance,
        tip: Balance,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;
}

#[async_trait::async_trait]
pub trait BalanceUserBatchExtApi {
    async fn batch_transfer(
        &self,
        dest: &[AccountId],
        amount: Balance,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;
}

#[async_trait::async_trait]
impl<C: ConnectionApi> BalanceApi for C {
    async fn locks_for_account(
        &self,
        account: AccountId,
        at: Option<BlockHash>,
    ) -> Vec<BalanceLock<Balance>> {
        let address = aleph_zero::api::storage().balances().locks(&account);

        self.get_storage_entry(&address, at).await.0
    }

    async fn locks(
        &self,
        accounts: &[AccountId],
        at: Option<BlockHash>,
    ) -> Vec<Vec<BalanceLock<Balance>>> {
        let mut locks = vec![];

        for account in accounts {
            locks.push(self.locks_for_account(account.clone(), at).await);
        }

        locks
    }

    async fn total_issuance(&self, at: Option<BlockHash>) -> Balance {
        let address = api::storage().balances().total_issuance();

        self.get_storage_entry(&address, at).await
    }
}

#[async_trait::async_trait]
impl<S: SignedConnectionApi> BalanceUserApi for S {
    async fn transfer(
        &self,
        dest: AccountId,
        amount: Balance,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash> {
        let tx = api::tx()
            .balances()
            .transfer(MultiAddress::Id(dest), amount);
        self.send_tx(tx, status).await
    }

    async fn transfer_with_tip(
        &self,
        dest: AccountId,
        amount: Balance,
        tip: Balance,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash> {
        let tx = api::tx()
            .balances()
            .transfer(MultiAddress::Id(dest), amount);

        self.send_tx_with_params(tx, PolkadotExtrinsicParamsBuilder::new().tip(tip), status)
            .await
    }
}

#[async_trait::async_trait]
impl<S: SignedConnectionApi> BalanceUserBatchExtApi for S {
    async fn batch_transfer(
        &self,
        dests: &[AccountId],
        amount: Balance,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash> {
        let calls = dests
            .iter()
            .map(|dest| {
                Balances(transfer {
                    dest: MultiAddress::Id(dest.clone()),
                    value: amount,
                })
            })
            .collect();
        self.batch_call(calls, status).await
    }
}
