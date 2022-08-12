use std::{default::Default, sync::Arc};

use sc_block_builder::BlockBuilderProvider;
use sc_client_api::HeaderBackend;
use sp_api::BlockId;
use sp_consensus::BlockOrigin;
use sp_core::hash::H256;
use sp_runtime::{traits::Block as BlockT, Digest};
use substrate_test_runtime_client::{
    runtime::{Block, Header},
    ClientBlockImportExt, ClientExt, TestClient,
};

use crate::BlockHashNum;

// A helper struct that allows to build blocks without importing/finalizing them right away.
pub struct ClientChainBuilder {
    pub client: Arc<TestClient>,
    // client_builder is used for the purpose of creating blocks only. It is necessary as we cannot create a block
    // in the "main" client without importing it.
    // We keep the invariant that all blocks are first created and imported by `client_builder` and only afterwards
    // can be possibly imported by `client`.
    pub client_builder: Arc<TestClient>,
    pub unique_seed: u32,
}

fn assert_no_blocks_except_genesis(client: &TestClient) {
    assert!(
        client.header(&BlockId::Number(1)).unwrap().is_none(),
        "Client is aware of some blocks beyond genesis"
    );
}

impl ClientChainBuilder {
    pub fn new(client: Arc<TestClient>, client_builder: Arc<TestClient>) -> Self {
        // Below we enforce that both clients are "empty" and agree with each other.
        assert_eq!(client.info(), client_builder.info());
        assert_no_blocks_except_genesis(&client);
        assert_no_blocks_except_genesis(&client_builder);
        ClientChainBuilder {
            client,
            client_builder,
            unique_seed: 0,
        }
    }

    /// Import block in test client
    pub async fn import_block(&mut self, block: Block) {
        self.client
            .import(BlockOrigin::Own, block.clone())
            .await
            .unwrap();
    }

    /// Finalize block with given hash without providing justification.
    pub fn finalize_block(&self, hash: &H256) {
        self.client
            .finalize_block(BlockId::Hash(*hash), None)
            .unwrap();
    }

    pub fn genesis_hash_num(&self) -> BlockHashNum<Block> {
        BlockHashNum::<Block>::new(self.client.info().genesis_hash, 0u64)
    }

    pub fn genesis_hash(&self) -> H256 {
        self.genesis_hash_num().hash
    }

    pub fn get_unique_bytes(&mut self) -> Vec<u8> {
        self.unique_seed += 1;
        self.unique_seed.to_be_bytes().to_vec()
    }

    pub async fn build_block_above(&mut self, parent: &H256) -> Block {
        let unique_bytes: Vec<u8> = self.get_unique_bytes();
        let mut digest = Digest::default();
        digest.push(sp_runtime::generic::DigestItem::Other(unique_bytes));
        let block = self
            .client_builder
            .new_block_at(&BlockId::Hash(*parent), digest, false)
            .unwrap()
            .build()
            .unwrap()
            .block;

        self.client_builder
            .import(BlockOrigin::Own, block.clone())
            .await
            .unwrap();
        block
    }

    /// Builds a sequence of blocks extending from `hash` of length `len`
    pub async fn build_branch_above(&mut self, parent: &H256, len: usize) -> Vec<Block> {
        let mut blocks = Vec::new();
        let mut prev_hash = *parent;
        for _ in 0..len {
            let block = self.build_block_above(&prev_hash).await;
            prev_hash = block.hash();
            blocks.push(block);
        }
        blocks
    }

    /// imports a sequence of blocks, should be in correct order
    pub async fn import_branch(&mut self, blocks: Vec<Block>) {
        for block in blocks {
            self.import_block(block.clone()).await;
        }
    }

    /// Builds a sequence of blocks extending from `hash` of length `len` and imports them
    pub async fn build_and_import_branch_above(&mut self, parent: &H256, len: usize) -> Vec<Block> {
        let blocks = self.build_branch_above(parent, len).await;
        self.import_branch(blocks.clone()).await;
        blocks
    }

    pub fn get_header_at(&self, num: u64) -> Header {
        self.client_builder
            .header(&BlockId::Number(num))
            .unwrap()
            .unwrap()
    }

    /// Builds a sequence of blocks extending from genesis of length `len`
    pub async fn initialize_single_branch(&mut self, len: usize) -> Vec<Block> {
        self.build_branch_above(&self.genesis_hash(), len).await
    }

    /// Builds and imports a sequence of blocks extending from genesis of length `len`
    pub async fn initialize_single_branch_and_import(&mut self, len: usize) -> Vec<Block> {
        self.build_and_import_branch_above(&self.genesis_hash(), len)
            .await
    }
}
