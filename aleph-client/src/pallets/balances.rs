use subxt::ext::sp_runtime::MultiAddress;

use crate::{
    aleph_zero::{self, api, api::runtime_types::pallet_balances::BalanceLock},
    connections::TxInfo,
    pallet_balances::pallet::Call::transfer,
    pallets::utility::UtilityApi,
    AccountId, Balance, BlockHash,
    Call::Balances,
    ConnectionApi, ParamsBuilder, SignedConnectionApi, TxStatus,
};

/// Pallet balances read-only API.
#[async_trait::async_trait]
pub trait BalanceApi {
    /// API for [`locks`](https://paritytech.github.io/substrate/master/pallet_balances/pallet/struct.Pallet.html#method.locks) call.
    /// * `account` - an account to query locked balance for
    /// * `at` - optional hash of a block to query state from
    async fn locks_for_account(
        &self,
        account: AccountId,
        at: Option<BlockHash>,
    ) -> Vec<BalanceLock<Balance>>;

    /// API for [`locks`](https://paritytech.github.io/substrate/master/pallet_balances/pallet/struct.Pallet.html#method.locks) call.
    /// * `accounts` - a list of accounts to query locked balance for
    /// * `at` - optional hash of a block to query state from
    async fn locks(
        &self,
        accounts: &[AccountId],
        at: Option<BlockHash>,
    ) -> Vec<Vec<BalanceLock<Balance>>>;

    /// Returns [`total_issuance`](https://paritytech.github.io/substrate/master/pallet_balances/pallet/type.TotalIssuance.html).
    async fn total_issuance(&self, at: Option<BlockHash>) -> Balance;
}

/// Pallet balances API
#[async_trait::async_trait]
pub trait BalanceUserApi {
    /// API for [`transfer`](https://paritytech.github.io/substrate/master/pallet_balances/pallet/struct.Pallet.html#method.transfer) call.
    async fn transfer(
        &self,
        dest: AccountId,
        amount: Balance,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo>;

    /// API for [`transfer`](https://paritytech.github.io/substrate/master/pallet_balances/pallet/struct.Pallet.html#method.transfer) call.
    /// Include tip in the tx.
    async fn transfer_with_tip(
        &self,
        dest: AccountId,
        amount: Balance,
        tip: Balance,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo>;
}

/// Pallet balances logic not directly related to any pallet call.
#[async_trait::async_trait]
pub trait BalanceUserBatchExtApi {
    /// Performs batch of `balances.transfer` calls.
    /// * `dest` - a list of accounts to send tokens to
    /// * `amount` - an amount to transfer
    /// * `status` - a [`TxStatus`] for a tx to wait for
    ///
    /// # Examples
    /// ```ignore
    ///  for chunk in stash_accounts.chunks(1024) {
    ///         connection
    ///             .batch_transfer(chunk, 1_000_000_000_000u128, TxStatus::InBlock)
    ///             .await
    ///             .unwrap();
    ///     }
    /// ```
    async fn batch_transfer(
        &self,
        dest: &[AccountId],
        amount: Balance,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo>;
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
    ) -> anyhow::Result<TxInfo> {
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
    ) -> anyhow::Result<TxInfo> {
        let tx = api::tx()
            .balances()
            .transfer(MultiAddress::Id(dest), amount);

        self.send_tx_with_params(tx, ParamsBuilder::new().tip(tip), status)
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
    ) -> anyhow::Result<TxInfo> {
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
