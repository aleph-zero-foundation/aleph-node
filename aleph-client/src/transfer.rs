use crate::{send_xt, Connection};
use codec::Compact;
use sp_core::Pair;
use sp_runtime::MultiAddress;
use substrate_api_client::{
    compose_call, compose_extrinsic, AccountId, GenericAddress, UncheckedExtrinsicV4, XtStatus,
};

pub type TransferTransaction =
    UncheckedExtrinsicV4<([u8; 2], MultiAddress<AccountId, ()>, Compact<u128>)>;

pub fn transfer(
    connection: &Connection,
    target: &AccountId,
    value: u128,
    status: XtStatus,
) -> TransferTransaction {
    let xt = connection.balance_transfer(GenericAddress::Id(target.clone()), value);
    send_xt(connection, xt.hex_encode(), "transfer", status);
    xt
}

pub fn batch_transfer(connection: &Connection, account_keys: Vec<AccountId>, endowment: u128) {
    let batch_endow: Vec<_> = account_keys
        .into_iter()
        .map(|account_id| {
            compose_call!(
                connection.metadata,
                "Balances",
                "transfer",
                GenericAddress::Id(account_id),
                Compact(endowment)
            )
        })
        .collect();

    let xt = compose_extrinsic!(connection, "Utility", "batch", batch_endow);
    send_xt(
        connection,
        xt.hex_encode(),
        "batch of endow balances",
        XtStatus::InBlock,
    );
}
