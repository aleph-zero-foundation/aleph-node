use aleph_primitives::BlockNumber;
use sp_api::BlockId;
use sp_blockchain::Info;
use sp_runtime::traits::Block;

use crate::{
    testing::mocks::{TBlock, THash, THeader},
    BlockchainBackend,
};
#[derive(Clone)]
pub struct Backend {
    blocks: Vec<TBlock>,
    next_block_to_finalize: TBlock,
}

pub fn create_block(parent_hash: THash, number: BlockNumber) -> TBlock {
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

impl Backend {
    pub fn new(finalized_height: BlockNumber) -> Self {
        let mut blocks: Vec<TBlock> = vec![];

        for n in 1..=finalized_height {
            let parent_hash = match n {
                1 => GENESIS_HASH.into(),
                _ => blocks.last().unwrap().header.hash(),
            };
            blocks.push(create_block(parent_hash, n));
        }

        let next_block_to_finalize =
            create_block(blocks.last().unwrap().hash(), finalized_height + 1);

        Backend {
            blocks,
            next_block_to_finalize,
        }
    }

    pub fn next_block_to_finalize(&self) -> TBlock {
        self.next_block_to_finalize.clone()
    }

    pub fn get_block(&self, id: BlockId<TBlock>) -> Option<TBlock> {
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

impl BlockchainBackend<TBlock> for Backend {
    fn children(&self, parent_hash: THash) -> Vec<THash> {
        if self.next_block_to_finalize.hash() == parent_hash {
            Vec::new()
        } else if self
            .blocks
            .last()
            .map(|b| b.hash())
            .unwrap()
            .eq(&parent_hash)
        {
            vec![self.next_block_to_finalize.hash()]
        } else {
            self.blocks
                .windows(2)
                .flat_map(<&[TBlock; 2]>::try_from)
                .find(|[parent, _]| parent.header.hash().eq(&parent_hash))
                .map(|[_, c]| vec![c.hash()])
                .unwrap_or_default()
        }
    }
    fn header(&self, id: BlockId<TBlock>) -> sp_blockchain::Result<Option<THeader>> {
        Ok(self.get_block(id).map(|b| b.header))
    }
    fn info(&self) -> Info<TBlock> {
        Info {
            best_hash: self.next_block_to_finalize.hash(),
            best_number: self.next_block_to_finalize.header.number,
            finalized_hash: self.blocks.last().unwrap().hash(),
            finalized_number: self.blocks.len() as BlockNumber,
            genesis_hash: GENESIS_HASH.into(),
            number_leaves: Default::default(),
            finalized_state: None,
            block_gap: None,
        }
    }
}

unsafe impl Send for Backend {}

unsafe impl Sync for Backend {}
