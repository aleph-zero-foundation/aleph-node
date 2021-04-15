use sp_consensus::{BlockCheckParams, BlockImport, BlockImportParams, ImportResult};
use sp_runtime::traits::Block as BlockT;
use std::{collections::HashMap, marker::PhantomData};

pub struct AlephBlockImport<Block: BlockT, I: BlockImport<Block>> {
    inner: I,
    _phantom: PhantomData<Block>,
}

impl<Block: BlockT, I: BlockImport<Block>> AlephBlockImport<Block, I> {
    pub fn new(inner: I) -> AlephBlockImport<Block, I> {
        AlephBlockImport {
            inner,
            _phantom: PhantomData,
        }
    }
}

impl<Block: BlockT, I: BlockImport<Block> + Clone> Clone for AlephBlockImport<Block, I> {
    fn clone(&self) -> Self {
        AlephBlockImport {
            inner: self.inner.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<Block: BlockT, I: BlockImport<Block>> BlockImport<Block> for AlephBlockImport<Block, I> {
    type Error = I::Error;
    type Transaction = I::Transaction;

    fn check_block(&mut self, block: BlockCheckParams<Block>) -> Result<ImportResult, Self::Error> {
        self.inner.check_block(block)
    }

    fn import_block(
        &mut self,
        block: BlockImportParams<Block, Self::Transaction>,
        cache: HashMap<[u8; 4], Vec<u8>>,
    ) -> Result<ImportResult, Self::Error> {
        self.inner.import_block(block, cache)
    }
}
