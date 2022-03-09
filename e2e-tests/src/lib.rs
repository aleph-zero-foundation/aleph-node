use codec::Compact;
use sp_runtime::MultiAddress;
use substrate_api_client::{AccountId, UncheckedExtrinsicV4};

mod accounts;
pub mod config;
mod fee;
mod staking;
pub mod test;
mod transfer;
mod waiting;

type TransferTransaction =
    UncheckedExtrinsicV4<([u8; 2], MultiAddress<AccountId, ()>, Compact<u128>)>;

#[macro_export]
macro_rules! send_extrinsic {
	($connection: expr,
	$module: expr,
	$call: expr,
    $exit_on: expr,
    $hash_log: expr
	$(, $args: expr) *) => {
		{
            use substrate_api_client::{compose_extrinsic, UncheckedExtrinsicV4};

            let tx: UncheckedExtrinsicV4<_> = compose_extrinsic!(
                $connection,
                $module,
                $call
                $(, ($args)) *
            );

            let tx_hash = $connection
                .send_extrinsic(tx.hex_encode(), $exit_on)
                .expect("Could not send extrinsic")
                .expect("Could not get tx hash");
            $hash_log(tx_hash);

            tx
		}
    };
}

#[macro_export]
macro_rules! send_extrinsic_no_wait {
	($connection: expr,
	$module: expr,
	$call: expr
	$(, $args: expr) *) => {
		{
            use substrate_api_client::{compose_extrinsic, UncheckedExtrinsicV4, XtStatus};

            let tx: UncheckedExtrinsicV4<_> = compose_extrinsic!(
                $connection,
                $module,
                $call
                $(, ($args)) *
            );

            let _ = $connection
                .send_extrinsic(tx.hex_encode(), XtStatus::InBlock)
                .unwrap();
		}
    };
}
