use std::{thread::sleep, time::Duration};

use codec::Encode;
use log::{info, warn};
use sp_core::{sr25519, Pair, H256};
use sp_core::storage::StorageKey;
use sp_runtime::{generic::Header as GenericHeader, traits::BlakeTwo256};
use substrate_api_client::{
    rpc::ws_client::WsRpcClient, std::error::Error, AccountId, Api, ApiResult, RpcClient,
    UncheckedExtrinsicV4, XtStatus,
};

pub use account::{get_free_balance, locks};
pub use debug::print_storages;
pub use fee::{get_next_fee_multiplier, get_tx_fee_info, FeeInfo};
pub use multisig::{
    compute_call_hash, perform_multisig_with_threshold_1, MultisigError, MultisigParty,
    SignatureAggregation,
};
pub use rpc::{rotate_keys, rotate_keys_raw_result, state_query_storage_at};
pub use session::{
    change_members, get_current as get_current_session, set_keys, wait_for as wait_for_session,
    Keys as SessionKeys,
};
pub use staking::{
    batch_bond as staking_batch_bond, batch_nominate as staking_batch_nominate,
    bond as staking_bond, bonded as staking_bonded, force_new_era as staking_force_new_era,
    ledger as staking_ledger, nominate as staking_nominate, payout_stakers,
    payout_stakers_and_assert_locked_balance, set_staking_limits as staking_set_staking_limits,
    validate as staking_validate, wait_for_full_era_completion, wait_for_next_era,
};
pub use substrate_api_client;
pub use system::set_code;
pub use transfer::{
    batch_transfer as balances_batch_transfer, transfer as balances_transfer, TransferTransaction,
};
pub use waiting::{wait_for_event, wait_for_finalized_block};

mod account;
mod debug;
mod fee;
mod multisig;
mod rpc;
mod session;
mod staking;
mod system;
mod transfer;
mod waiting;

pub trait FromStr: Sized {
    type Err;

    fn from_str(s: &str) -> Result<Self, Self::Err>;
}

impl FromStr for WsRpcClient {
    type Err = ();

    fn from_str(url: &str) -> Result<Self, Self::Err> {
        Ok(WsRpcClient::new(url))
    }
}

pub type BlockNumber = u32;
pub type Header = GenericHeader<BlockNumber, BlakeTwo256>;
pub type KeyPair = sr25519::Pair;
pub type Connection = Api<KeyPair, WsRpcClient>;

pub fn create_connection(address: &str) -> Connection {
    create_custom_connection(address).expect("Connection should be created")
}

enum Protocol {
    Ws,
    Wss,
}

impl Default for Protocol {
    fn default() -> Self {
        Protocol::Ws
    }
}

impl ToString for Protocol {
    fn to_string(&self) -> String {
        match self {
            Protocol::Ws => String::from("ws://"),
            Protocol::Wss => String::from("wss://"),
        }
    }
}

/// Unless `address` already contains protocol, we prepend to it `ws://`.
fn ensure_protocol(address: &str) -> String {
    if address.starts_with(&Protocol::Ws.to_string())
        || address.starts_with(&Protocol::Wss.to_string())
    {
        return address.to_string();
    }
    format!("{}{}", Protocol::default().to_string(), address)
}

pub fn create_custom_connection<Client: FromStr + RpcClient>(
    address: &str,
) -> Result<Api<sr25519::Pair, Client>, <Client as FromStr>::Err> {
    loop {
        let client = Client::from_str(&ensure_protocol(address))?;
        match Api::<sr25519::Pair, _>::new(client) {
            Ok(api) => return Ok(api),
            Err(why) => {
                warn!(
                    "[+] Can't create_connection because {:?}, will try again in 1s",
                    why
                );
                sleep(Duration::from_millis(1000));
            }
        }
    }
}

/// `panic`able utility wrapper for `try_send_xt`.
pub fn send_xt<T: Encode>(
    connection: &Connection,
    xt: UncheckedExtrinsicV4<T>,
    xt_name: Option<&'static str>,
    xt_status: XtStatus,
) -> Option<H256> {
    try_send_xt(connection, xt, xt_name, xt_status).expect("Should manage to send extrinsic")
}

/// Sends transaction `xt` using `connection`. If `tx_status` is either `Finalized` or `InBlock`
/// additionally returns hash of the containing block. `xt_name` is used only for logging purposes.
/// Recoverable.
pub fn try_send_xt<T: Encode>(
    connection: &Connection,
    xt: UncheckedExtrinsicV4<T>,
    xt_name: Option<&'static str>,
    xt_status: XtStatus,
) -> ApiResult<Option<H256>> {
    let hash = connection
        .send_extrinsic(xt.hex_encode(), xt_status)?
        .ok_or_else(|| Error::Other(String::from("Could not get tx/block hash").into()))?;

    match xt_status {
        XtStatus::Finalized | XtStatus::InBlock => {
            info!(target: "aleph-client",
                "Transaction `{}` was included in block with hash {}.",
                xt_name.unwrap_or_default(), hash);
            Ok(Some(hash))
        }
        // Other variants either do not return (see https://github.com/scs/substrate-api-client/issues/175)
        // or return xt hash, which is kinda useless here.
        _ => Ok(None),
    }
}

pub fn keypair_from_string(seed: &str) -> KeyPair {
    KeyPair::from_string(seed, None).expect("Can't create pair from seed value")
}

pub fn account_from_keypair(keypair: &KeyPair) -> AccountId {
    AccountId::from(keypair.public())
}

fn storage_key(module: &str, version: &str) -> [u8; 32] {
    let pallet_name = sp_core::hashing::twox_128(module.as_bytes());
    let postfix = sp_core::hashing::twox_128(version.as_bytes());
    let mut final_key = [0u8; 32];
    final_key[..16].copy_from_slice(&pallet_name);
    final_key[16..].copy_from_slice(&postfix);
    final_key
}

/// Computes hash of given pallet's call. You can use that to pass result to `state.getKeys` RPC call.
/// * `pallet` name of the pallet
/// * `call` name of the pallet's call
///
/// # Example
/// ```
/// let staking_nominate_storage_key = get_storage_key("Staking", "Nominators");
/// assert_eq!(staking_nominate_storage_key, String::from("5f3e4907f716ac89b6347d15ececedca9c6a637f62ae2af1c7e31eed7e96be04"));
/// ```
pub fn get_storage_key(pallet: &str, call: &str) -> String {
    let bytes = storage_key(pallet, call);
    let storage_key = StorageKey(bytes.into());
    hex::encode(storage_key.0)
}
