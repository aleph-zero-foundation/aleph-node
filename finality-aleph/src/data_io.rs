use aleph_primitives::ALEPH_ENGINE_ID;
use codec::Encode;
use log::{debug, error};
use rush::OrderedBatch;
use sc_client_api::backend::Backend;
use sp_consensus::SelectChain;
use sp_runtime::{
    generic::BlockId,
    traits::{Block, Header},
    Justification,
};
use std::{marker::PhantomData, sync::Arc};

use crate::{AuthorityKeystore, Error};
use futures::{channel::mpsc, StreamExt};

use crate::justification::AlephJustification;
use sp_api::NumberFor;

pub(crate) struct BlockFinalizer<C, B: Block, BE> {
    client: Arc<C>,
    auth_keystore: AuthorityKeystore,
    ordered_batch_rx: mpsc::UnboundedReceiver<OrderedBatch<B::Hash>>,
    phantom: PhantomData<BE>,
}

impl<C, B: Block, BE> BlockFinalizer<C, B, BE>
where
    BE: Backend<B>,
    C: crate::ClientForAleph<B, BE>,
{
    pub(crate) fn new(
        client: Arc<C>,
        auth_keystore: AuthorityKeystore,
        ordered_batch_rx: mpsc::UnboundedReceiver<OrderedBatch<B::Hash>>,
    ) -> Self {
        BlockFinalizer {
            client,
            auth_keystore,
            ordered_batch_rx,
            phantom: PhantomData,
        }
    }

    fn check_extends_finalized(&self, h: B::Hash) -> bool {
        let head_finalized = self.client.info().finalized_hash;
        if h == head_finalized {
            return false;
        }
        let lca = sp_blockchain::lowest_common_ancestor(self.client.as_ref(), h, head_finalized)
            .expect("No lowest common ancestor");
        lca.hash == head_finalized
    }

    fn finalize_block(&self, h: B::Hash) {
        let block_number = match self.client.number(h) {
            Ok(Some(number)) => number,
            _ => {
                error!(target: "afa", "a block with hash {} should already be in chain", h);
                return;
            }
        };
        finalize_block(
            self.client.clone(),
            h,
            block_number,
            Some((
                ALEPH_ENGINE_ID,
                AlephJustification::new::<B>(&self.auth_keystore, h).encode(),
            )),
        );
    }

    pub(crate) async fn run(mut self) {
        while let Some(batch) = self.ordered_batch_rx.next().await {
            for block_hash in batch {
                if self.check_extends_finalized(block_hash) {
                    self.finalize_block(block_hash);
                    debug!(target: "afa", "Finalized block hash {}.", block_hash);
                }
            }
        }
        error!(target: "afa", "Voter batch stream closed.");
    }
}

#[derive(Clone)]
pub(crate) struct DataIO<B: Block, SC: SelectChain<B>> {
    pub(crate) select_chain: SC,
    pub(crate) ordered_batch_tx: mpsc::UnboundedSender<OrderedBatch<B::Hash>>,
}

impl<B: Block, SC: SelectChain<B>> rush::DataIO<B::Hash> for DataIO<B, SC> {
    type Error = Error;

    fn get_data(&self) -> B::Hash {
        self.select_chain
            .best_chain()
            .expect("No best chain")
            .hash()
    }

    fn send_ordered_batch(&mut self, batch: OrderedBatch<B::Hash>) -> Result<(), Self::Error> {
        // TODO: add better conversion
        self.ordered_batch_tx
            .unbounded_send(batch)
            .map_err(|_| Error::SendData)
    }
}

pub(crate) fn finalize_block<BE, B, C>(
    client: Arc<C>,
    hash: B::Hash,
    block_number: NumberFor<B>,
    justification: Option<Justification>,
) where
    B: Block,
    BE: Backend<B>,
    C: crate::ClientForAleph<B, BE>,
{
    let info = client.info();

    if info.finalized_number >= block_number {
        error!(target: "afa", "trying to finalized a block with hash {} and number {}
               that is not greater than already finalized {}", hash, block_number, info.finalized_number);
        return;
    }

    let status = client.info();
    debug!(target: "afa", "Finalizing block with hash {:?}. Previous best: #{:?}.", hash, status.finalized_number);

    let _update_res = client.lock_import_and_run(|import_op| {
        // NOTE: all other finalization logic should come here, inside the lock
        client.apply_finality(import_op, BlockId::Hash(hash), justification, true)
    });

    let status = client.info();
    debug!(target: "afa", "Finalized block with hash {:?}. Current best: #{:?}.", hash, status.finalized_number);
}
