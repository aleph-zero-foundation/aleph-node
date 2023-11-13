use core::result::Result;
use std::{marker::PhantomData, sync::Arc, time::Instant};

use log::{debug, warn};
use sc_client_api::{Backend, Finalizer, HeaderBackend, LockImportRun};
use sp_blockchain::Error;
use sp_runtime::{
    traits::{Block, Header},
    Justification,
};

use crate::{
    aleph_primitives::{BlockHash, BlockNumber},
    metrics::Checkpoint,
    BlockId, TimingBlockMetrics,
};

pub trait BlockFinalizer {
    fn finalize_block(&self, block: BlockId, justification: Justification) -> Result<(), Error>;
}

pub struct AlephFinalizer<B, BE, C>
where
    B: Block,
    BE: Backend<B>,
    C: HeaderBackend<B> + LockImportRun<B, BE> + Finalizer<B, BE>,
{
    client: Arc<C>,
    metrics: TimingBlockMetrics,
    phantom: PhantomData<(B, BE)>,
}

impl<B, BE, C> AlephFinalizer<B, BE, C>
where
    B: Block,
    BE: Backend<B>,
    C: HeaderBackend<B> + LockImportRun<B, BE> + Finalizer<B, BE>,
{
    pub(crate) fn new(client: Arc<C>, metrics: TimingBlockMetrics) -> Self {
        AlephFinalizer {
            client,
            metrics,
            phantom: PhantomData,
        }
    }
}

impl<B, BE, C> BlockFinalizer for AlephFinalizer<B, BE, C>
where
    B: Block<Hash = BlockHash>,
    B::Header: Header<Number = BlockNumber>,
    BE: Backend<B>,
    C: HeaderBackend<B> + LockImportRun<B, BE> + Finalizer<B, BE>,
{
    fn finalize_block(&self, block: BlockId, justification: Justification) -> Result<(), Error> {
        let number = block.number();
        let hash = block.hash();

        let status = self.client.info();
        if status.finalized_number >= number {
            warn!(target: "aleph-finality", "trying to finalize a block with hash {} and number {}
               that is not greater than already finalized {}", hash, number, status.finalized_number);
        }

        debug!(target: "aleph-finality", "Finalizing block with hash {:?} and number {:?}. Previous best: #{:?}.", hash, number, status.finalized_number);

        let update_res = self.client.lock_import_and_run(|import_op| {
            // NOTE: all other finalization logic should come here, inside the lock
            self.client
                .apply_finality(import_op, hash, Some(justification), true)
        });

        let status = self.client.info();
        match &update_res {
            Ok(_) => {
                debug!(target: "aleph-finality", "Successfully finalized block with hash {:?} and number {:?}. Current best: #{:?}.", hash, number, status.best_number);
                self.metrics
                    .report_block(hash, Instant::now(), Checkpoint::Finalized);
            }
            Err(_) => {
                debug!(target: "aleph-finality", "Failed to finalize block with hash {:?} and number {:?}. Current best: #{:?}.", hash, number, status.best_number)
            }
        }

        update_res
    }
}
