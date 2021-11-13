use log::warn;
use sp_core::sr25519;
use std::thread::sleep;
use std::time::Duration;
use substrate_api_client::rpc::WsRpcClient;
use substrate_api_client::Api;

pub fn create_connection(url: String) -> Api<sr25519::Pair, WsRpcClient> {
    let client = WsRpcClient::new(&format!("ws://{}", url));
    match Api::<sr25519::Pair, _>::new(client) {
        Ok(api) => api,
        Err(why) => {
            warn!(
                "[+] Can't create_connection because {:?}, will try again in 1s",
                why
            );
            sleep(Duration::from_millis(1000));
            create_connection(url)
        }
    }
}
