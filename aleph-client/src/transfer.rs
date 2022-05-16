use crate::{send_xt, AnyConnection, SignedConnection};
use codec::Compact;
use sp_core::Pair;
use sp_runtime::MultiAddress;
use substrate_api_client::{
    compose_call, compose_extrinsic, AccountId, GenericAddress, UncheckedExtrinsicV4, XtStatus,
};

pub type TransferTransaction =
    UncheckedExtrinsicV4<([u8; 2], MultiAddress<AccountId, ()>, Compact<u128>)>;

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
