use log::{info, warn};
use sp_core::sr25519;
use sp_runtime::{generic::Header as GenericHeader, traits::BlakeTwo256};
use std::{thread::sleep, time::Duration};
use substrate_api_client::{rpc::ws_client::WsRpcClient, Api, RpcClient, XtStatus};

mod rpc;
mod session;
mod staking;
mod waiting;

pub use rpc::rotate_keys;
pub use session::{
    change_members, get_current as get_current_session, set_keys, wait_for as wait_for_session,
    Keys as SessionKeys,
};
pub use staking::bond as staking_bond;
pub use waiting::wait_for_event;

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

#[derive(Copy, Clone, Debug)]
pub enum Protocol {
    WS,
    WSS,
}

impl Default for Protocol {
    fn default() -> Self {
        Protocol::WS
    }
}

pub fn from(use_ssl: bool) -> Protocol {
    match use_ssl {
        true => Protocol::WSS,
        false => Protocol::WS,
    }
}

pub fn create_connection(address: &str, protocol: Protocol) -> Connection {
    create_custom_connection(address, protocol).expect("connection should be created")
}

pub fn create_custom_connection<Client: FromStr + RpcClient>(
    address: &str,
    protocol: Protocol,
) -> Result<Api<sr25519::Pair, Client>, <Client as FromStr>::Err> {
    let protocol = match protocol {
        Protocol::WS => "ws",
        Protocol::WSS => "wss",
    };
    loop {
        let client = Client::from_str(&format!("{}://{}", protocol, address))?;
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
