use std::{collections::HashMap, iter::repeat_with, sync::Arc};

use async_channel::Receiver;
use futures::{future::join_all, join};
use log::info;
use parking_lot::Mutex;

use crate::{
    jsonrpc_client::Client,
    types::{BlockHash, StorageKey},
    Storage,
};

pub struct StateFetcher {
    client: Client,
}

impl StateFetcher {
    pub async fn new(ws_rpc_endpoint: String) -> Self {
        StateFetcher {
            client: Client::new(&ws_rpc_endpoint).await.unwrap(),
        }
    }

    async fn value_fetching_worker(
        &self,
        block: BlockHash,
        input: Receiver<StorageKey>,
        output: Arc<Mutex<Storage>>,
    ) {
        const LOG_PROGRESS_FREQUENCY: usize = 500;

        while let Ok(key) = input.recv().await {
            let value = self
                .client
                .get_storage(key.clone(), block.clone())
                .await
                .unwrap();

            let mut output_guard = output.lock();
            output_guard.insert(key, value);
            if output_guard.len() % LOG_PROGRESS_FREQUENCY == 0 {
                info!("Fetched {} values", output_guard.len());
            }
        }
    }

    async fn get_full_state_at_block(&self, block_hash: BlockHash, num_workers: u32) -> Storage {
        info!("Fetching state at block {:?}", block_hash);

        let (input, key_fetcher) = self.client.stream_all_keys(&block_hash);
        let output = Arc::new(Mutex::new(HashMap::new()));

        let workers = repeat_with(|| {
            self.value_fetching_worker(block_hash.clone(), input.clone(), output.clone())
        })
        .take(num_workers as usize);

        info!("Started {} workers to download values.", num_workers);
        let (res, _) = join!(key_fetcher, join_all(workers));
        res.unwrap();

        Arc::try_unwrap(output).unwrap().into_inner()
    }

    pub async fn get_full_state(&self, at_block: Option<BlockHash>, num_workers: u32) -> Storage {
        let block = match at_block {
            None => self.client.best_block().await.unwrap(),
            Some(block) => block,
        };

        self.get_full_state_at_block(block, num_workers).await
    }
}
