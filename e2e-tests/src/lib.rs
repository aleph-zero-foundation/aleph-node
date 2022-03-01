use codec::Compact;
use log::info;
use sp_core::sr25519;
use sp_runtime::{generic, traits::BlakeTwo256, MultiAddress};
use substrate_api_client::rpc::WsRpcClient;
use substrate_api_client::{AccountId, Api, UncheckedExtrinsicV4, XtStatus};

mod accounts;
pub mod config;
mod fee;
pub mod rpc;
pub mod session;
mod staking;
pub mod test;
mod transfer;
mod waiting;

type BlockNumber = u32;
type Header = generic::Header<BlockNumber, BlakeTwo256>;
type KeyPair = sr25519::Pair;
type Connection = Api<KeyPair, WsRpcClient>;
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

pub fn send_xt(connection: &Connection, xt: String, xt_name: &'static str, tx_status: XtStatus) {
    let block_hash = connection
        .send_extrinsic(xt, tx_status)
        .expect("Could not send extrinsic")
        .expect("Could not get tx hash");
    let block_number = connection
        .get_header::<Header>(Some(block_hash))
        .expect("Could not fetch header")
        .expect("Block exists; qed")
        .number;
    info!(
        "Transaction {} was included in block {}.",
        xt_name, block_number
    );
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
