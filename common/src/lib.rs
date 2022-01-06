mod ws_rpc_client;

use log::warn;
use sp_core::sr25519;
use std::{thread::sleep, time::Duration};
use substrate_api_client::{Api, RpcClient};
pub use ws_rpc_client::WsRpcClient;

pub trait FromStr {
    fn from_str(s: &str) -> Self;
}

impl FromStr for substrate_api_client::rpc::ws_client::WsRpcClient {
    fn from_str(url: &str) -> Self {
        substrate_api_client::rpc::ws_client::WsRpcClient::new(url)
    }
}

impl FromStr for WsRpcClient {
    fn from_str(url: &str) -> Self {
        WsRpcClient::new(url)
    }
}

pub fn create_connection(
    address: String,
) -> Api<sr25519::Pair, substrate_api_client::rpc::ws_client::WsRpcClient> {
    create_custom_connection(&address)
}

pub fn create_custom_connection<Client: FromStr + RpcClient>(
    address: &str,
) -> Api<sr25519::Pair, Client> {
    let client = Client::from_str(&format!("ws://{}", address));
    match Api::<sr25519::Pair, _>::new(client) {
        Ok(api) => api,
        Err(why) => {
            warn!(
                "[+] Can't create_connection because {:?}, will try again in 1s",
                why
            );
            sleep(Duration::from_millis(1000));
            create_custom_connection(address)
        }
    }
}
