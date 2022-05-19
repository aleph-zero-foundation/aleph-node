use std::collections::HashMap;

use clap::Parser;
use env_logger::Env;
use log::info;
use serde_json::Value;

use crate::{
    chainspec_combining::combine_states,
    config::{Config, StoragePath},
    fetching::StateFetcher,
    fsio::{read_json_from_file, read_snapshot_from_file, save_snapshot_to_file, write_to_file},
};

mod chainspec_combining;
mod config;
mod fetching;
mod fsio;

type BlockHash = String;
type StorageKey = String;
type StorageValue = String;
type Storage = HashMap<StorageKey, StorageValue>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Config {
        http_rpc_endpoint,
        initial_spec_path,
        snapshot_path,
        combined_spec_path,
        use_snapshot_file,
        storage_keep_state,
        num_workers,
    } = Config::parse();
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    info!(target: "fork",
        "Running with config: \n\
        \thttp_rpc_endpoint: {}\n\
        \tinitial_spec_path: {}\n\
        \tsnapshot_path: {}\n\
        \tcombined_spec_path: {}\n\
        \tuse_snapshot_file: {}\n\
        \tstorage_keep_state: {:?}",
        &http_rpc_endpoint, &initial_spec_path, &snapshot_path, &combined_spec_path, &use_snapshot_file, &storage_keep_state
    );

    let mut initial_spec: Value = read_json_from_file(initial_spec_path);

    assert_ne!(
        initial_spec["genesis"]["raw"],
        Value::Null,
        "The initial provided chainspec must be raw! Make sure you use --raw when generating it."
    );

    if !use_snapshot_file {
        let fetcher = StateFetcher::new(http_rpc_endpoint);
        let state = fetcher.get_full_state_at_best_block(num_workers).await;
        save_snapshot_to_file(state, snapshot_path.clone());
    }
    let state = read_snapshot_from_file(snapshot_path);

    let initial_state: Storage =
        serde_json::from_value(initial_spec["genesis"]["raw"]["top"].take())
            .expect("Deserialization of state from given chainspec file failed");

    let state = combine_states(state, initial_state, storage_keep_state);
    let json_state = serde_json::to_value(state).expect("Failed to convert a storage map to json");
    initial_spec["genesis"]["raw"]["top"] = json_state;
    let new_spec = serde_json::to_vec_pretty(&initial_spec)?;

    info!(target: "fork", "Writing new chainspec to {}", &combined_spec_path);
    write_to_file(combined_spec_path, &new_spec);

    info!("Done!");
    Ok(())
}
