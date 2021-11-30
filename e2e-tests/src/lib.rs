use codec::Compact;
use sp_core::sr25519;
use sp_runtime::{generic, traits::BlakeTwo256, MultiAddress};
use substrate_api_client::rpc::WsRpcClient;
use substrate_api_client::{AccountId, Api, UncheckedExtrinsicV4};

mod accounts;
pub mod config;
mod fee;
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
    $hash_log: expr
	$(, $args: expr) *) => {
		{
            use substrate_api_client::{compose_extrinsic, UncheckedExtrinsicV4, XtStatus};

            let tx: UncheckedExtrinsicV4<_> = compose_extrinsic!(
                $connection,
                $module,
                $call
                $(, ($args)) *
            );

            let tx_hash = $connection
                .send_extrinsic(tx.hex_encode(), XtStatus::Finalized)
                .unwrap()
                .expect("Could not get tx hash");
            $hash_log(tx_hash);

            tx
		}
    };
}
