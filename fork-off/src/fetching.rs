use std::{collections::HashMap, sync::Arc};

use futures::future::join_all;
use log::info;
use parking_lot::Mutex;
use reqwest::Client;
use serde_json::Value;

use crate::{BlockHash, Storage, StorageKey, StorageValue};

const KEYS_BATCH_SIZE: u32 = 1000;

pub struct StateFetcher {
    client: Client,
    http_rpc_endpoint: String,
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

fn block_hash_body(block_num: Option<u32>) -> Value {
    let params = serde_json::json!([block_num]);
    construct_json_body("chain_getBlockHash", params)
}

fn get_storage_value_body(key: StorageKey, block_hash: Option<BlockHash>) -> Value {
    let params = serde_json::json!([key, block_hash]);
    construct_json_body("state_getStorage", params)
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

impl StateFetcher {
    pub fn new(http_rpc_endpoint: String) -> Self {
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
