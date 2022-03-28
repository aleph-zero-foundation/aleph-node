use clap::Parser;
use env_logger::Env;
use futures::future::join_all;
use log::info;
use parking_lot::Mutex;
use reqwest::Client;
use serde_json::Value;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{ErrorKind, Write},
    sync::Arc,
};

#[derive(Debug, Parser)]
#[clap(version = "1.0")]
pub struct Config {
    /// URL address of the node RPC endpoint for the chain you are forking
    #[clap(long, default_value = "http://127.0.0.1:9933")]
    pub http_rpc_endpoint: String,

    /// path of the initial chainspec (generated with the `bootstrap-chain` command)
    #[clap(long, default_value = "./initial_chainspec.json")]
    pub initial_spec_path: String,

    /// where to write the snapshot of the state
    #[clap(long, default_value = "./snapshot.json")]
    pub snapshot_path: String,

    /// where to write the forked genesis chainspec
    #[clap(long, default_value = "./chainspec_from_snapshot.json")]
    pub combined_spec_path: String,

    /// where to write the forked genesis chainspec
    #[clap(long)]
    pub use_snapshot_file: bool,

    /// how many parallel processes to download values -- note that large values might result in bans because
    /// of rate-limiting mechanisms
    #[clap(long, default_value_t = 5)]
    pub num_workers: u32,

    /// which modules to set in forked spec
    #[clap(
        long,
        multiple_occurrences = true,
        takes_value = true,
        value_delimiter = ',',
        default_value = "Aura,Aleph,Sudo,Staking,Session,Elections"
    )]
    pub pallets_keep_state: Vec<String>,
}

const KEYS_BATCH_SIZE: u32 = 1000;

type BlockHash = String;
type StorageKey = String;
type StorageValue = String;
type Storage = HashMap<StorageKey, StorageValue>;

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

fn block_hash_body(block_num: Option<u32>) -> Value {
    let params = serde_json::json!([block_num]);
    construct_json_body("chain_getBlockHash", params)
}

fn get_keys_paged_body(
    prefix: StorageKey,
    count: u32,
    start_key: Option<StorageKey>,
    block_hash: Option<BlockHash>,
) -> Value {
    let params = serde_json::json!([prefix, count, start_key, block_hash]);
    construct_json_body("state_getKeysPaged", params)
}

fn get_storage_value_body(key: StorageKey, block_hash: Option<BlockHash>) -> Value {
    let params = serde_json::json!([key, block_hash]);
    construct_json_body("state_getStorage", params)
}

struct StateFetcher {
    client: Client,
    http_rpc_endpoint: String,
}

impl StateFetcher {
    fn new(http_rpc_endpoint: String) -> Self {
        StateFetcher {
            client: Client::new(),
            http_rpc_endpoint,
        }
    }

    async fn value_fetching_worker(
        http_rpc_endpoint: String,
        id: usize,
        block: BlockHash,
        input: Arc<Mutex<Vec<StorageKey>>>,
        output: Arc<Mutex<Storage>>,
    ) {
        const LOG_PROGRESS_FREQUENCY: usize = 500;
        let fetcher = StateFetcher::new(http_rpc_endpoint);
        loop {
            let maybe_key = {
                let mut keys = input.lock();
                keys.pop()
            };
            let key = match maybe_key {
                Some(key) => key,
                None => break,
            };
            let value = fetcher.get_value(key.clone(), block.clone()).await;
            let mut output_guard = output.lock();
            output_guard.insert(key, value);
            if output_guard.len() % LOG_PROGRESS_FREQUENCY == 0 {
                info!("Fetched {} values", output_guard.len());
            }
        }
        info!("Worker {:?} finished", id);
    }

    async fn make_request<T: serde::de::DeserializeOwned>(&self, body: Value) -> T {
        make_request_and_parse_result::<T>(&self.client, &self.http_rpc_endpoint, body).await
    }

    pub async fn get_most_recent_block(&self) -> BlockHash {
        let body = block_hash_body(None);
        let block: String = self.make_request(body).await;
        block
    }

    // The start_key is not included in the range. The result is `count` keys that appear after `start_key` in the
    // lexicographic ordering of all keys in storage.
    async fn get_keys_range(
        &self,
        count: u32,
        start_key: Option<StorageKey>,
        block_hash: Option<BlockHash>,
    ) -> Vec<StorageKey> {
        let prefix = String::from("");
        let body = get_keys_paged_body(prefix, count, start_key, block_hash);
        let keys: Vec<StorageKey> = self.make_request(body).await;
        keys
    }

    async fn get_all_keys(&self, block_hash: BlockHash) -> Vec<StorageKey> {
        let mut last_key_fetched = None;
        let mut all_keys = Vec::new();
        loop {
            let new_keys = self
                .get_keys_range(KEYS_BATCH_SIZE, last_key_fetched, Some(block_hash.clone()))
                .await;
            all_keys.extend_from_slice(&new_keys);
            info!(
                "Got {} new keys and have {} in total",
                new_keys.len(),
                all_keys.len()
            );
            if new_keys.len() < KEYS_BATCH_SIZE as usize {
                break;
            }
            last_key_fetched = Some(new_keys.last().unwrap().clone());
        }
        all_keys
    }

    async fn get_value(&self, key: StorageKey, block_hash: BlockHash) -> StorageValue {
        let body = get_storage_value_body(key, Some(block_hash));
        self.make_request(body).await
    }

    async fn get_values(
        &self,
        keys: Vec<StorageKey>,
        block_hash: BlockHash,
        num_workers: u32,
    ) -> Storage {
        let n_keys = keys.len();
        let input = Arc::new(Mutex::new(keys));
        let output = Arc::new(Mutex::new(HashMap::with_capacity(n_keys)));
        let mut workers = Vec::new();

        for id in 0..(num_workers as usize) {
            workers.push(StateFetcher::value_fetching_worker(
                self.http_rpc_endpoint.clone(),
                id,
                block_hash.clone(),
                input.clone(),
                output.clone(),
            ));
        }
        info!("Started {} workers to download values.", workers.len());
        join_all(workers).await;
        assert!(input.lock().is_empty(), "Not all keys were fetched");
        let mut guard = output.lock();
        std::mem::take(&mut guard)
    }

    pub async fn get_full_state_at(
        &self,
        num_workers: u32,
        fetch_block: Option<BlockHash>,
    ) -> Storage {
        let block = if let Some(block) = fetch_block {
            block
        } else {
            self.get_most_recent_block().await
        };
        info!("Fetching state at block {:?}", block);
        let keys = self.get_all_keys(block.clone()).await;
        self.get_values(keys, block, num_workers).await
    }

    pub async fn get_full_state_at_best_block(&self, num_workers: u32) -> Storage {
        self.get_full_state_at(num_workers, None).await
    }
}

fn save_snapshot_to_file(snapshot: Storage, path: String) {
    let data = serde_json::to_vec_pretty(&snapshot).unwrap();
    info!(
        "Writing snapshot of {} key-val pairs and {} total bytes",
        snapshot.len(),
        data.len()
    );
    write_to_file(path, &data);
}

fn read_snapshot_from_file(path: String) -> Storage {
    let snapshot: Storage =
        serde_json::from_str(&fs::read_to_string(&path).expect("Could not read snapshot file"))
            .expect("could not parse from snapshot");
    info!("Read snapshot of {} key-val pairs", snapshot.len());
    snapshot
}

fn is_prefix_of(shorter: &str, longer: &str) -> bool {
    longer.starts_with(shorter)
}

fn combine_states(mut state: Storage, initial_state: Storage, pallets: Vec<String>) -> Storage {
    let pallets_prefixes: Vec<(String, String)> = pallets
        .iter()
        .map(|pallet| {
            let hash = format!("0x{}", prefix_as_hex(pallet));
            (pallet.clone(), hash)
        })
        .collect();
    let mut removed_per_pallet_count: HashMap<String, usize> = pallets_prefixes
        .iter()
        .map(|(pallet, _)| (pallet.clone(), 0))
        .collect();
    let mut added_per_pallet_cnt = removed_per_pallet_count.clone();
    state.retain(|k, _v| {
        match pallets_prefixes
            .iter()
            .find(|(_, prefix)| is_prefix_of(prefix, k))
        {
            Some((pallet, _)) => {
                *removed_per_pallet_count.get_mut(pallet).unwrap() += 1;
                false
            }
            None => true,
        }
    });
    for (k, v) in initial_state.iter() {
        if let Some((pallet, _)) = pallets_prefixes
            .iter()
            .find(|(_, prefix)| is_prefix_of(prefix, k))
        {
            *added_per_pallet_cnt.get_mut(pallet).unwrap() += 1;
            state.insert(k.clone(), v.clone());
        }
    }
    for (pallet, prefix) in pallets_prefixes {
        info!(
            "For pallet {} (prefix {}) Replaced {} entries by {} entries from initial_spec",
            pallet, prefix, removed_per_pallet_count[&pallet], added_per_pallet_cnt[&pallet]
        );
    }
    state
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Config {
        http_rpc_endpoint,
        initial_spec_path,
        snapshot_path,
        combined_spec_path,
        use_snapshot_file,
        pallets_keep_state,
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
        \tpallets_keep_state: {:?}",
        &http_rpc_endpoint, &initial_spec_path, &snapshot_path, &combined_spec_path, &use_snapshot_file, &pallets_keep_state
    );

    let mut initial_spec: Value = serde_json::from_str(
        &fs::read_to_string(&initial_spec_path).expect("Could not read chainspec file"),
    )?;

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
    let state = combine_states(state, initial_state, pallets_keep_state);
    let json_state = serde_json::to_value(state).expect("Failed to convert a storage map to json");
    initial_spec["genesis"]["raw"]["top"] = json_state;
    let new_spec = serde_json::to_vec_pretty(&initial_spec)?;
    info!(target: "fork", "Writing new chainspec to {}", &combined_spec_path);
    write_to_file(combined_spec_path, &new_spec);

    info!("Done!");
    Ok(())
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
