use crate::{send_xt, Connection};
use codec::Compact;
use sp_runtime::MultiAddress;
use substrate_api_client::{AccountId, GenericAddress, UncheckedExtrinsicV4, XtStatus};

pub type TransferTransaction =
    UncheckedExtrinsicV4<([u8; 2], MultiAddress<AccountId, ()>, Compact<u128>)>;

pub fn transfer(
    connection: &Connection,
    target: &AccountId,
    value: u128,
    status: XtStatus,
) -> TransferTransaction {
    let xt = connection.balance_transfer(GenericAddress::Id(target.clone()), value);
    send_xt(&connection, xt.hex_encode(), "transfer", status);
    xt
}
