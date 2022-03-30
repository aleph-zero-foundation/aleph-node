use std::{thread::sleep, time::Duration};

use log::{info, warn};
use sp_core::{sr25519, Pair};
use sp_runtime::{generic::Header as GenericHeader, traits::BlakeTwo256};
use substrate_api_client::{rpc::ws_client::WsRpcClient, Api, RpcClient, XtStatus};

pub use account::{get_free_balance, get_locked_balance, locks};
pub use fee::{get_next_fee_multiplier, get_tx_fee_info, FeeInfo};
pub use rpc::{rotate_keys, rotate_keys_raw_result};
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
pub use system::set_code;
pub use transfer::{
    batch_transfer as balances_batch_transfer, transfer as balances_transfer, TransferTransaction,
};
pub use waiting::{wait_for_event, wait_for_finalized_block};

mod account;
mod fee;
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
    WS,
    WSS,
}

impl Default for Protocol {
    fn default() -> Self {
        Protocol::WS
    }
}

impl ToString for Protocol {
    fn to_string(&self) -> String {
        match self {
            Protocol::WS => String::from("ws://"),
            Protocol::WSS => String::from("wss://"),
        }
    }
}

/// Unless `address` already contains protocol, we prepend to it `ws://`.
fn ensure_protocol(address: &str) -> String {
    if address.starts_with(&Protocol::WS.to_string())
        || address.starts_with(&Protocol::WSS.to_string())
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
        target: "aleph-client",
        "Transaction {} was included in block {}.",
        xt_name, block_number
    );
}

pub fn keypair_from_string(seed: &str) -> KeyPair {
    KeyPair::from_string(seed, None).expect("Can't create pair from seed value")
}
