use core::result::Result;
use std::{marker::PhantomData, sync::Arc};

use log::{debug, warn};
use sc_client_api::{Backend, Finalizer, HeaderBackend, LockImportRun};
use sp_api::NumberFor;
use sp_blockchain::Error;
use sp_runtime::{traits::Block, Justification};

pub trait BlockFinalizer<B: Block> {
    fn finalize_block(
        &self,
        hash: B::Hash,
        block_number: NumberFor<B>,
        justification: Option<Justification>,
    ) -> Result<(), Error>;
}

pub struct AlephFinalizer<B, BE, C>
where
    B: Block,
    BE: Backend<B>,
    C: HeaderBackend<B> + LockImportRun<B, BE> + Finalizer<B, BE>,
{
    client: Arc<C>,
    phantom: PhantomData<(B, BE)>,
}

impl<B, BE, C> AlephFinalizer<B, BE, C>
where
    B: Block,
    BE: Backend<B>,
    C: HeaderBackend<B> + LockImportRun<B, BE> + Finalizer<B, BE>,
{
    pub(crate) fn new(client: Arc<C>) -> Self {
        AlephFinalizer {
            client,
            phantom: PhantomData,
        }
    }
}

impl<B, BE, C> BlockFinalizer<B> for AlephFinalizer<B, BE, C>
where
    B: Block,
    BE: Backend<B>,
    C: HeaderBackend<B> + LockImportRun<B, BE> + Finalizer<B, BE>,
{
    fn finalize_block(
        &self,
        hash: B::Hash,
        block_number: NumberFor<B>,
        justification: Option<Justification>,
    ) -> Result<(), Error> {
        let status = self.client.info();
        if status.finalized_number >= block_number {
            warn!(target: "aleph-finality", "trying to finalize a block with hash {} and number {}
               that is not greater than already finalized {}", hash, block_number, status.finalized_number);
        }

        debug!(target: "aleph-finality", "Finalizing block with hash {:?} and number {:?}. Previous best: #{:?}.", hash, block_number, status.finalized_number);

        let update_res = self.client.lock_import_and_run(|import_op| {
            // NOTE: all other finalization logic should come here, inside the lock
            self.client
                .apply_finality(import_op, hash, justification, true)
        });
        let status = self.client.info();
        debug!(target: "aleph-finality", "Attempted to finalize block with hash {:?}. Current best: #{:?}.", hash, status.finalized_number);
        update_res
    }
}
