use codec::Compact;
use primitives::Balance;
use sp_core::{Pair, H256};
use sp_runtime::MultiAddress;
use substrate_api_client::{
    compose_call, compose_extrinsic, compose_extrinsic_offline, error::Error as SacError,
    AccountId, ExtrinsicParams, GenericAddress, XtStatus,
};

use crate::{
    send_xt, try_send_xt, AnyConnection, BalanceTransfer, BatchTransactions, Extrinsic,
    SignedConnection,
};

pub type TransferCall = ([u8; 2], MultiAddress<AccountId, ()>, Compact<u128>);
pub type TransferTransaction = Extrinsic<TransferCall>;

pub fn transfer(
    connection: &SignedConnection,
    target: &AccountId,
    value: u128,
    status: XtStatus,
) -> TransferTransaction {
    let xt = connection
        .as_connection()
        .balance_transfer(GenericAddress::Id(target.clone()), value);
    send_xt(connection, xt.clone(), Some("transfer"), status);
    xt
}

pub fn batch_transfer(
    connection: &SignedConnection,
    account_keys: Vec<AccountId>,
    endowment: u128,
) {
    let batch_endow = account_keys
        .into_iter()
        .map(|account_id| {
            compose_call!(
                connection.as_connection().metadata,
                "Balances",
                "transfer",
                GenericAddress::Id(account_id),
                Compact(endowment)
            )
        })
        .collect::<Vec<_>>();

    let xt = compose_extrinsic!(connection.as_connection(), "Utility", "batch", batch_endow);
    send_xt(
        connection,
        xt,
        Some("batch of endow balances"),
        XtStatus::InBlock,
    );
}

impl SignedConnection {
    pub fn create_transfer_extrinsic(
        &self,
        tx: <Self as BalanceTransfer>::TransferTx,
    ) -> TransferTransaction {
        let nonce = self.as_connection().get_nonce().unwrap();
        compose_extrinsic_offline!(
            self.as_connection().signer.unwrap(),
            tx,
            self.as_connection().extrinsic_params(nonce)
        )
    }
}

impl BalanceTransfer for SignedConnection {
    type TransferTx = TransferCall;
    type Error = SacError;

    fn create_transfer_tx(&self, account: AccountId, amount: Balance) -> Self::TransferTx {
        compose_call!(
            self.as_connection().metadata,
            "Balances",
            "transfer",
            GenericAddress::Id(account),
            amount.into()
        )
    }

    fn transfer(
        &self,
        tx: Self::TransferTx,
        status: XtStatus,
    ) -> Result<Option<H256>, Self::Error> {
        let xt = self.create_transfer_extrinsic(tx);
        try_send_xt(self, xt, Some("transfer"), status)
    }
}

impl BatchTransactions<<SignedConnection as BalanceTransfer>::TransferTx> for SignedConnection {
    type Error = SacError;

    fn batch_and_send_transactions<'a>(
        &self,
        transactions: impl IntoIterator<Item = &'a <SignedConnection as BalanceTransfer>::TransferTx>,
        status: XtStatus,
    ) -> Result<Option<H256>, Self::Error>
    where
        <SignedConnection as BalanceTransfer>::TransferTx: 'a,
    {
        let txs = Vec::from_iter(transactions);
        let xt = compose_extrinsic!(self.as_connection(), "Utility", "batch", txs);
        try_send_xt(self, xt, Some("batch/transfer"), status)
    }
}
