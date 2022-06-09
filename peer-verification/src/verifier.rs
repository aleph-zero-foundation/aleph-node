use clap::Parser;
use libp2p::{identity::PublicKey, PeerId};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{str::FromStr, u64};

type BlockNumber = u64;

#[derive(Debug, Parser)]
#[clap(version = "1.0")]
pub struct Config {
    // Node to ask for network state
    #[clap(long, default_value = "http://127.0.0.1:9933")]
    pub node: String,

    /// Peer Id of the peer we want to verify.
    #[clap(long)]
    pub peer_id: String,

    /// Message that the signature of we want to check.
    #[clap(long)]
    pub message: String,

    /// Hex encoded public key of the peer.
    #[clap(long)]
    pub public_key: String,

    /// Signature we want to check.
    #[clap(long)]
    pub signature: String,

    /// Max block difference with head.
    #[clap(long, default_value = "10")]
    pub block_difference: BlockNumber,
}

async fn make_request_and_parse_result<T: serde::de::DeserializeOwned>(
    client: &Client,
    endpoint: &str,
    body: Value,
) -> T {
    let mut response: Value = client
        .post(endpoint)
        .json(&body)
        .send()
        .await
        .expect("Storage request has failed")
        .json()
        .await
        .expect("Could not deserialize response as JSON");
    let result = response["result"].take();
    serde_json::from_value(result).expect("Incompatible type of the result")
}

fn construct_json_body(method_name: &str, params: Value) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method_name,
        "params": params
    })
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PeerState {
    best_number: BlockNumber,
    peer_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct BlockHeader {
    number: String,
}

impl BlockHeader {
    fn number(&self) -> Result<BlockNumber, std::num::ParseIntError> {
        // For some reason in this rpc call result is returned in hex encoded decimal, so such ugly conversion is needed
        BlockNumber::from_str_radix(self.number.trim_start_matches("0x"), 16)
    }
}

async fn get_connected_peers(client: &Client, node: &str) -> Vec<PeerState> {
    let req = construct_json_body("system_peers", Value::Null);
    make_request_and_parse_result(client, node, req).await
}

async fn get_best_block_header(client: &Client, node: &str) -> BlockHeader {
    let block_req = construct_json_body("chain_getHeader", Value::Null);
    make_request_and_parse_result(client, node, block_req).await
}

#[tokio::main]
async fn main() {
    let Config {
        node,
        peer_id,
        message,
        public_key,
        signature,
        block_difference,
    } = Config::parse();

    let peer_id = PeerId::from_str(&peer_id).unwrap();

    let public_key = PublicKey::from_protobuf_encoding(
        &hex::decode(public_key).expect("Could not decode public key from hex encoding"),
    )
    .expect("Could not decode public key from protobuf encoding");
    let signature = hex::decode(signature).unwrap();

    assert_eq!(
        public_key.to_peer_id(),
        peer_id,
        "Supplied public key inconsistent with peer id"
    );
    assert!(
        public_key.verify(message.as_bytes(), &signature),
        "Supplied signature is incorrect"
    );

    let client = Client::new();
    let connected_peers = get_connected_peers(&client, &node).await;
    let best_block_header = get_best_block_header(&client, &node).await;

    let best_number = best_block_header.number().unwrap();

    let peer = connected_peers
        .iter()
        .find(|r| r.peer_id == peer_id.to_string())
        .unwrap_or_else(|| panic!("No peer with peer id {:?} connected", peer_id));

    assert!(
        best_number <= peer.best_number + block_difference,
        "Peer is not up to date. Should have {:?} block number. Has {:?} block number",
        best_number,
        peer.best_number
    );

    assert!(
        peer.best_number <= best_number + block_difference,
        "Peer is too far in the future. Should have {:?} block number. Has {:?} block number",
        best_number,
        peer.best_number
    );

    println!(
        "Signature for peer {} is correct and peer is up to date with block creation at {:?}",
        peer_id, best_number
    );
}
