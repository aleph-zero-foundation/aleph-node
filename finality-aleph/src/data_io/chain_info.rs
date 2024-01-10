use std::marker::PhantomData;

use log::{debug, info};
use lru::LruCache;

use crate::{
    aleph_primitives::{BlockHash, BlockNumber},
    block::{Header, HeaderBackend},
    data_io::ChainInfoCacheConfig,
    BlockId,
};

const LOG_TARGET: &str = "aleph-data-store";

pub trait ChainInfoProvider: Send + Sync + 'static {
    fn is_block_imported(&mut self, block: &BlockId) -> bool;

    fn get_finalized_at(&mut self, number: BlockNumber) -> Result<BlockId, ()>;

    fn get_parent_hash(&mut self, block: &BlockId) -> Result<BlockHash, ()>;

    fn get_highest_finalized(&mut self) -> BlockId;
}

pub struct SubstrateChainInfoProvider<H, HB>
where
    H: Header,
    HB: HeaderBackend<H> + 'static,
{
    client: HB,
    _phantom: PhantomData<H>,
}

impl<H, HB> SubstrateChainInfoProvider<H, HB>
where
    H: Header,
    HB: HeaderBackend<H>,
{
    pub fn new(client: HB) -> Self {
        SubstrateChainInfoProvider {
            client,
            _phantom: PhantomData,
        }
    }
}
impl<H, HB> ChainInfoProvider for SubstrateChainInfoProvider<H, HB>
where
    H: Header,
    HB: HeaderBackend<H>,
{
    fn is_block_imported(&mut self, block: &BlockId) -> bool {
        match self.client.header(block) {
            Ok(maybe_header) => maybe_header.is_some(),
            Err(e) => {
                debug!(
                    target: LOG_TARGET,
                    "Error while fetching header in ChainInfoProvider: {:?}", e
                );
                false
            }
        }
    }

    fn get_finalized_at(&mut self, number: BlockNumber) -> Result<BlockId, ()> {
        match self.client.header_of_finalized_at(number) {
            Ok(Some(header)) => Ok(header.id()),
            _ => Err(()),
        }
    }

    fn get_parent_hash(&mut self, block: &BlockId) -> Result<BlockHash, ()> {
        match self.client.header(block) {
            Ok(Some(header)) => Ok(header.parent_id().ok_or(())?.hash()),
            Ok(None) => {
                info!(
                    target: LOG_TARGET,
                    "Block not found while getting parent hash in ChainInfoProvider"
                );
                Err(())
            }
            Err(e) => {
                info!(
                    target: LOG_TARGET,
                    "Error while getting parent hash in ChainInfoProvider: {:?}", e
                );
                Err(())
            }
        }
    }

    fn get_highest_finalized(&mut self) -> BlockId {
        self.client.top_finalized_id()
    }
}

pub struct CachedChainInfoProvider<CIP>
where
    CIP: ChainInfoProvider,
{
    available_block_with_parent_cache: LruCache<BlockId, BlockHash>,
    available_blocks_cache: LruCache<BlockId, ()>,
    finalized_cache: LruCache<BlockNumber, BlockHash>,
    chain_info_provider: CIP,
}

impl<CIP> CachedChainInfoProvider<CIP>
where
    CIP: ChainInfoProvider,
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

impl<CIP> ChainInfoProvider for CachedChainInfoProvider<CIP>
where
    CIP: ChainInfoProvider,
{
    fn is_block_imported(&mut self, block: &BlockId) -> bool {
        if self.available_blocks_cache.contains(block) {
            return true;
        }

        if self.chain_info_provider.is_block_imported(block) {
            self.available_blocks_cache.put(block.clone(), ());
            return true;
        }
        false
    }

    fn get_finalized_at(&mut self, num: BlockNumber) -> Result<BlockId, ()> {
        if let Some(hash) = self.finalized_cache.get(&num) {
            return Ok((*hash, num).into());
        }

        if self.get_highest_finalized().number() < num {
            return Err(());
        }

        if let Ok(block) = self.chain_info_provider.get_finalized_at(num) {
            self.finalized_cache.put(num, block.hash());
            return Ok(block);
        }
        Err(())
    }

    fn get_parent_hash(&mut self, block: &BlockId) -> Result<BlockHash, ()> {
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

    fn get_highest_finalized(&mut self) -> BlockId {
        self.chain_info_provider.get_highest_finalized()
    }
}

// A wrapper around any ChainInfoProvider that uses auxiliary information on finalization `aux_finalized`
// and considers as finalized a block that is either finalized in the sense of the inner ChainInfoProvider
// or is <= the `aux_finalized` block.
// `aux_finalized` is supposed to be updated using `update_aux_finalized`.
pub struct AuxFinalizationChainInfoProvider<CIP>
where
    CIP: ChainInfoProvider,
{
    aux_finalized: BlockId,
    chain_info_provider: CIP,
}

impl<CIP> AuxFinalizationChainInfoProvider<CIP>
where
    CIP: ChainInfoProvider,
{
    pub fn new(chain_info_provider: CIP, aux_finalized: BlockId) -> Self {
        AuxFinalizationChainInfoProvider {
            aux_finalized,
            chain_info_provider,
        }
    }

    pub fn update_aux_finalized(&mut self, aux_finalized: BlockId) {
        self.aux_finalized = aux_finalized;
    }
}

impl<CIP> ChainInfoProvider for AuxFinalizationChainInfoProvider<CIP>
where
    CIP: ChainInfoProvider,
{
    fn is_block_imported(&mut self, block: &BlockId) -> bool {
        self.chain_info_provider.is_block_imported(block)
    }

    fn get_finalized_at(&mut self, num: BlockNumber) -> Result<BlockId, ()> {
        let highest_finalized_inner = self.chain_info_provider.get_highest_finalized();
        if num <= highest_finalized_inner.number() {
            return self.chain_info_provider.get_finalized_at(num);
        }
        if num > self.aux_finalized.number() {
            return Err(());
        }
        // We are in the situation: internal_highest_finalized < num <= aux_finalized
        let mut curr_block = self.aux_finalized.clone();
        while curr_block.number() > num {
            let parent_hash = self.chain_info_provider.get_parent_hash(&curr_block)?;
            curr_block = (parent_hash, curr_block.number() - 1).into();
        }
        Ok(curr_block)
    }

    fn get_parent_hash(&mut self, block: &BlockId) -> Result<BlockHash, ()> {
        self.chain_info_provider.get_parent_hash(block)
    }

    fn get_highest_finalized(&mut self) -> BlockId {
        let highest_finalized_inner = self.chain_info_provider.get_highest_finalized();
        if self.aux_finalized.number() > highest_finalized_inner.number() {
            self.aux_finalized.clone()
        } else {
            highest_finalized_inner
        }
    }
}
