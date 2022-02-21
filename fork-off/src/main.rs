use clap::Parser;
use futures::stream::{self, StreamExt};
use serde_json::Value;
use std::fs::{self, File};
use std::io::{ErrorKind, Write};
use substrate_api_client::extrinsic::log::{debug, info};

#[derive(Debug, Parser)]
#[clap(version = "1.0")]
pub struct Config {
    /// URL address of the node RPC endpoint for the chain you are forking
    #[clap(long, default_value = "http://127.0.0.1:9933")]
    pub http_rpc_endpoint: String,

    /// path to write the initial chainspec of the fork
    /// as generated with the `bootstrap-chain` command
    #[clap(long, default_value = "../docker/data/chainspec.json")]
    pub fork_spec_path: String,

    /// where to write the forked genesis chainspec
    #[clap(long, default_value = "../docker/data/chainspec.fork.json")]
    pub write_to_path: String,

    /// which modules to set in forked spec
    #[clap(
        long,
        multiple_occurrences = true,
        takes_value = true,
        value_delimiter = ',',
        default_value = "Aura,Aleph"
    )]
    pub prefixes: Vec<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Config {
        http_rpc_endpoint,
        fork_spec_path,
        write_to_path,
        prefixes,
    } = Config::parse();

    env_logger::init();

    info!(target: "fork",
        "Running with config: \n\thttp_rpc_endpoint {}\n \tfork_spec_path: {}\n \twrite_to_path: {}\n \tprefixes: {:?}",
        &http_rpc_endpoint, &fork_spec_path, &write_to_path, &prefixes
    );

    let mut fork_spec: Value = serde_json::from_str(
        &fs::read_to_string(&fork_spec_path).expect("Could not read chainspec file"),
    )?;

    let hashed_prefixes = prefixes
        .iter()
        .map(|prefix| {
            let hash = format!("0x{}", prefix_as_hex(prefix));
            info!(target: "fork", "prefix: {}, hash: {}", prefix, hash);
            hash
        })
        .chain([format!("0x{}", hex::encode(":code"))])
        .collect::<Vec<String>>();

    let storage = get_chain_state(&http_rpc_endpoint, &hashed_prefixes).await;

    info!("Succesfully retrieved chain state {:?}", storage);

    storage.into_iter().for_each(|(key, value)| {
        info!(target: "fork","Moving {} to the fork", key);
        fork_spec["genesis"]["raw"]["top"][key] = value.into();
    });

    // write out the fork spec
    let json = serde_json::to_string(&fork_spec)?;
    info!(target: "fork", "Writing forked chain spec to {}", &write_to_path);
    write_to_file(write_to_path, json.as_bytes());

    info!("Done!");
    Ok(())
}

async fn get_key(http_rpc_endpoint: &str, key: &str, start_key: Option<&str>) -> Option<String> {
    let body = match start_key {
        Some(start_key) => {
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "state_getKeysPaged",
                "params": [ key, 1, start_key ]
            })
        }
        None => serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "state_getKeysPaged",
            "params": [ key, 1 ]
        }),
    };

    let response: Value = reqwest::Client::new()
        .post(http_rpc_endpoint)
        .json(&body)
        .send()
        .await
        .expect("Storage request has failed")
        .json()
        .await
        .expect("Could not deserialize response as JSON");

    debug!(target: "fork", "get_key response: {}", response);

    let result = response["result"]
        .as_array()
        .expect("No result in response");

    if !result.is_empty() {
        return Some(
            result
                .first()
                .expect("No key in result")
                .as_str()
                .expect("Not a string")
                .to_owned(),
        );
    }
    None
}

async fn get_value(http_rpc_endpoint: &str, key: &str) -> String {
    let response: Value = reqwest::Client::new()
        .post(http_rpc_endpoint)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "state_getStorage",
            "params": [ &key ]
        }))
        .send()
        .await
        .expect("Storage request has failed")
        .json()
        .await
        .expect("Could not deserialize response as JSON");

    debug!(target: "fork", "get_value response: {}", response);

    response["result"]
        .as_str()
        .expect("Not a string")
        .to_owned()
}

async fn get_chain_state(
    http_rpc_endpoint: &str,
    hashed_prefixes: &[String],
) -> Vec<(String, String)> {
    stream::iter(hashed_prefixes.to_owned())
        .then(|prefix| async move {
            // collect storage pairs for this prefix
            let mut pairs = vec![];
            let mut first_key = get_key(http_rpc_endpoint, &prefix, None).await;
            debug!(target: "fork", "hashed prefix: {}, first key: {:?}", &prefix, &first_key);

            while let Some(key) = first_key {
                let value = get_value(http_rpc_endpoint, &key).await;
                pairs.push((key.clone(), value));
                first_key = get_key(http_rpc_endpoint, &prefix, Some(&key)).await;
                debug!(target: "fork", "hashed prefix: {}, next key: {:?}", &prefix, &first_key);
            }

            stream::iter(pairs)
        })
        .flatten()
        .collect()
        .await
}

fn write_to_file(write_to_path: String, data: &[u8]) {
    let mut file = match fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&write_to_path)
    {
        Ok(file) => file,
        Err(error) => match error.kind() {
            ErrorKind::NotFound => match File::create(&write_to_path) {
                Ok(file) => file,
                Err(why) => panic!("Cannot create file: {:?}", why),
            },
            _ => panic!("Unexpected error when creating file: {}", &write_to_path),
        },
    };

    file.write_all(data).expect("Could not write to file");
}

fn prefix_as_hex(module: &str) -> String {
    let pallet_name = sp_io::hashing::twox_128(module.as_bytes());
    hex::encode(pallet_name)
}
