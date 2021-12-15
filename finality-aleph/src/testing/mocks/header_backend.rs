use sp_api::BlockId;
use sp_blockchain::{BlockStatus, HeaderBackend, Info};
use sp_runtime::traits::Block;

use crate::testing::mocks::{TBlock, THash, THeader, TNumber};

#[derive(Clone)]
pub(crate) struct Client {
    blocks: Vec<TBlock>,
    next_block_to_finalize: TBlock,
}

pub(crate) fn create_block(parent_hash: THash, number: TNumber) -> TBlock {
    TBlock {
        header: THeader {
            parent_hash,
            number,
            state_root: Default::default(),
            extrinsics_root: Default::default(),
            digest: Default::default(),
        },
        extrinsics: vec![],
    }
}

const GENESIS_HASH: [u8; 32] = [0u8; 32];

impl Client {
    pub(crate) fn new(finalized_height: u64) -> Self {
        let mut blocks: Vec<TBlock> = vec![];

        for n in 1u64..=finalized_height {
            let parent_hash = match n {
                1 => GENESIS_HASH.into(),
                _ => blocks.last().unwrap().header.hash(),
            };
            blocks.push(create_block(parent_hash, n));
        }

        let next_block_to_finalize =
            create_block(blocks.last().unwrap().hash(), finalized_height + 1);

        Client {
            blocks,
            next_block_to_finalize,
        }
    }

    pub(crate) fn next_block_to_finalize(&self) -> TBlock {
        self.next_block_to_finalize.clone()
    }

    pub(crate) fn get_block(&self, id: BlockId<TBlock>) -> Option<TBlock> {
        match id {
            BlockId::Hash(h) => {
                if self.next_block_to_finalize.hash() == h {
                    Some(self.next_block_to_finalize.clone())
                } else {
                    self.blocks.iter().find(|b| b.header.hash().eq(&h)).cloned()
                }
            }
            BlockId::Number(n) => {
                if self.next_block_to_finalize.header.number == n {
                    Some(self.next_block_to_finalize.clone())
                } else {
                    self.blocks.get((n - 1) as usize).cloned()
                }
            }
        }
    }
}

impl HeaderBackend<TBlock> for Client {
    fn header(&self, id: BlockId<TBlock>) -> sp_blockchain::Result<Option<THeader>> {
        Ok(self.get_block(id).map(|b| b.header))
    }

    fn info(&self) -> Info<TBlock> {
        Info {
            best_hash: self.next_block_to_finalize.hash(),
            best_number: self.next_block_to_finalize.header.number,
            finalized_hash: self.blocks.last().unwrap().hash(),
            finalized_number: self.blocks.len() as u64,
            genesis_hash: GENESIS_HASH.into(),
            number_leaves: Default::default(),
            finalized_state: None,
        }
    }

    fn status(&self, id: BlockId<TBlock>) -> sp_blockchain::Result<BlockStatus> {
        Ok(match self.get_block(id) {
            Some(_) => BlockStatus::InChain,
            _ => BlockStatus::Unknown,
        })
    }

    fn number(&self, hash: THash) -> sp_blockchain::Result<Option<TNumber>> {
        Ok(self.get_block(BlockId::hash(hash)).map(|b| b.header.number))
    }

    fn hash(&self, number: TNumber) -> sp_blockchain::Result<Option<THash>> {
        Ok(self.get_block(BlockId::Number(number)).map(|b| b.hash()))
    }
}

unsafe impl Send for Client {}

unsafe impl Sync for Client {}
