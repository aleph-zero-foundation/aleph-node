use std::sync::Arc;

use aleph_primitives::BlockNumber;
use log::error;
use lru::LruCache;
use sc_client_api::HeaderBackend;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT, NumberFor};

use crate::{data_io::ChainInfoCacheConfig, BlockHashNum};

pub trait ChainInfoProvider<B: BlockT> {
    fn is_block_imported(&mut self, block: &BlockHashNum<B>) -> bool;

    fn get_finalized_at(&mut self, number: NumberFor<B>) -> Result<BlockHashNum<B>, ()>;

    fn get_parent_hash(&mut self, block: &BlockHashNum<B>) -> Result<B::Hash, ()>;

    fn get_highest_finalized(&mut self) -> BlockHashNum<B>;
}

impl<C, B> ChainInfoProvider<B> for Arc<C>
where
    B: BlockT,
    B::Header: HeaderT<Number = BlockNumber>,
    C: HeaderBackend<B>,
{
    fn is_block_imported(&mut self, block: &BlockHashNum<B>) -> bool {
        let maybe_header = self.header(block.hash).expect("client must answer a query");
        if let Some(header) = maybe_header {
            // If the block number is incorrect, we treat as not imported.
            return *header.number() == block.num;
        }
        false
    }

    fn get_finalized_at(&mut self, num: NumberFor<B>) -> Result<BlockHashNum<B>, ()> {
        if self.info().finalized_number < num {
            return Err(());
        }

        let block_hash = match self.hash(num).ok().flatten() {
            None => {
                error!(target: "chain-info", "Could not get hash for block #{:?}", num);
                return Err(());
            }
            Some(h) => h,
        };

        if let Some(header) = self.header(block_hash).expect("client must respond") {
            Ok((header.hash(), num).into())
        } else {
            Err(())
        }
    }

    fn get_parent_hash(&mut self, block: &BlockHashNum<B>) -> Result<B::Hash, ()> {
        if let Some(header) = self.header(block.hash).expect("client must respond") {
            Ok(*header.parent_hash())
        } else {
            Err(())
        }
    }

    fn get_highest_finalized(&mut self) -> BlockHashNum<B> {
        let status = self.info();
        (status.finalized_hash, status.finalized_number).into()
    }
}

pub struct CachedChainInfoProvider<B, CIP>
where
    B: BlockT,
    CIP: ChainInfoProvider<B>,
{
    available_block_with_parent_cache: LruCache<BlockHashNum<B>, B::Hash>,
    available_blocks_cache: LruCache<BlockHashNum<B>, ()>,
    finalized_cache: LruCache<NumberFor<B>, B::Hash>,
    chain_info_provider: CIP,
}

impl<B, CIP> CachedChainInfoProvider<B, CIP>
where
    B: BlockT,
    CIP: ChainInfoProvider<B>,
{
    pub fn new(chain_info_provider: CIP, config: ChainInfoCacheConfig) -> Self {
        CachedChainInfoProvider {
            available_block_with_parent_cache: LruCache::new(config.block_cache_capacity),
            available_blocks_cache: LruCache::new(config.block_cache_capacity),
            finalized_cache: LruCache::new(config.block_cache_capacity),
            chain_info_provider,
        }
    }

    pub fn inner(&mut self) -> &mut CIP {
        &mut self.chain_info_provider
    }
}

impl<B, CIP> ChainInfoProvider<B> for CachedChainInfoProvider<B, CIP>
where
    B: BlockT,
    B::Header: HeaderT<Number = BlockNumber>,
    CIP: ChainInfoProvider<B>,
{
    fn is_block_imported(&mut self, block: &BlockHashNum<B>) -> bool {
        if self.available_blocks_cache.contains(block) {
            return true;
        }

        if self.chain_info_provider.is_block_imported(block) {
            self.available_blocks_cache.put(block.clone(), ());
            return true;
        }
        false
    }

    fn get_finalized_at(&mut self, num: NumberFor<B>) -> Result<BlockHashNum<B>, ()> {
        if let Some(hash) = self.finalized_cache.get(&num) {
            return Ok((*hash, num).into());
        }

        if self.get_highest_finalized().num < num {
            return Err(());
        }

        if let Ok(block) = self.chain_info_provider.get_finalized_at(num) {
            self.finalized_cache.put(num, block.hash);
            return Ok(block);
        }
        Err(())
    }

    fn get_parent_hash(&mut self, block: &BlockHashNum<B>) -> Result<B::Hash, ()> {
        if let Some(parent) = self.available_block_with_parent_cache.get(block) {
            return Ok(*parent);
        }

        if let Ok(parent) = self.chain_info_provider.get_parent_hash(block) {
            self.available_block_with_parent_cache
                .put(block.clone(), parent);
            return Ok(parent);
        }
        Err(())
    }

    fn get_highest_finalized(&mut self) -> BlockHashNum<B> {
        self.chain_info_provider.get_highest_finalized()
    }
}

// A wrapper around any ChainInfoProvider that uses auxiliary information on finalization `aux_finalized`
// and considers as finalized a block that is either finalized in the sense of the inner ChainInfoProvider
// or is <= the `aux_finalized` block.
// `aux_finalized` is supposed to be updated using `update_aux_finalized`.
pub struct AuxFinalizationChainInfoProvider<B, CIP>
where
    B: BlockT,
    CIP: ChainInfoProvider<B>,
{
    aux_finalized: BlockHashNum<B>,
    chain_info_provider: CIP,
}

impl<B, CIP> AuxFinalizationChainInfoProvider<B, CIP>
where
    B: BlockT,
    CIP: ChainInfoProvider<B>,
{
    pub fn new(chain_info_provider: CIP, aux_finalized: BlockHashNum<B>) -> Self {
        AuxFinalizationChainInfoProvider {
            aux_finalized,
            chain_info_provider,
        }
    }

    pub fn update_aux_finalized(&mut self, aux_finalized: BlockHashNum<B>) {
        self.aux_finalized = aux_finalized;
    }
}

impl<B, CIP> ChainInfoProvider<B> for AuxFinalizationChainInfoProvider<B, CIP>
where
    B: BlockT,
    B::Header: HeaderT<Number = BlockNumber>,
    CIP: ChainInfoProvider<B>,
{
    fn is_block_imported(&mut self, block: &BlockHashNum<B>) -> bool {
        self.chain_info_provider.is_block_imported(block)
    }

    fn get_finalized_at(&mut self, num: NumberFor<B>) -> Result<BlockHashNum<B>, ()> {
        let highest_finalized_inner = self.chain_info_provider.get_highest_finalized();
        if num <= highest_finalized_inner.num {
            return self.chain_info_provider.get_finalized_at(num);
        }
        if num > self.aux_finalized.num {
            return Err(());
        }
        // We are in the situation: internal_highest_finalized < num <= aux_finalized
        let mut curr_block = self.aux_finalized.clone();
        while curr_block.num > num {
            let parent_hash = self.chain_info_provider.get_parent_hash(&curr_block)?;
            curr_block = (parent_hash, curr_block.num - 1).into();
        }
        Ok(curr_block)
    }

    fn get_parent_hash(&mut self, block: &BlockHashNum<B>) -> Result<B::Hash, ()> {
        self.chain_info_provider.get_parent_hash(block)
    }

    fn get_highest_finalized(&mut self) -> BlockHashNum<B> {
        let highest_finalized_inner = self.chain_info_provider.get_highest_finalized();
        if self.aux_finalized.num > highest_finalized_inner.num {
            self.aux_finalized.clone()
        } else {
            highest_finalized_inner
        }
    }
}
