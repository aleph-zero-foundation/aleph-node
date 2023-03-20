use std::{default::Default, sync::Arc};

use aleph_primitives::BlockNumber;
use futures::channel::mpsc;
use log::{debug, error, warn};
use sc_client_api::HeaderBackend;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT, NumberFor, Zero};

use crate::{
    data_io::{
        chain_info::{AuxFinalizationChainInfoProvider, CachedChainInfoProvider},
        status_provider::get_proposal_status,
        AlephData, ChainInfoProvider,
    },
    mpsc::TrySendError,
    BlockHashNum, SessionBoundaries,
};

type InterpretersChainInfoProvider<B, C> =
    CachedChainInfoProvider<B, AuxFinalizationChainInfoProvider<B, Arc<C>>>;

/// Takes as input ordered `AlephData` from `AlephBFT` and pushes blocks that should be finalized
/// to an output channel. The other end of the channel is held by the aggregator whose goal is to
/// create multisignatures under the finalized blocks.
pub struct OrderedDataInterpreter<B, C>
where
    B: BlockT,
    B::Header: HeaderT<Number = BlockNumber>,
    C: HeaderBackend<B>,
{
    blocks_to_finalize_tx: mpsc::UnboundedSender<BlockHashNum<B>>,
    chain_info_provider: InterpretersChainInfoProvider<B, C>,
    last_finalized_by_aleph: BlockHashNum<B>,
    session_boundaries: SessionBoundaries,
}

fn get_last_block_prev_session<B, C>(
    session_boundaries: SessionBoundaries,
    mut client: Arc<C>,
) -> BlockHashNum<B>
where
    B: BlockT,
    B::Header: HeaderT<Number = BlockNumber>,
    C: HeaderBackend<B>,
{
    if session_boundaries.first_block() > 0 {
        // We are in session > 0, we take the last block of previous session.
        let last_prev_session_num = session_boundaries.first_block() - 1;
        client.get_finalized_at(last_prev_session_num).expect(
            "Last block of previous session must have been finalized before starting the current",
        )
    } else {
        // We are in session 0, we take the genesis block -- it is finalized by definition.
        client
            .get_finalized_at(NumberFor::<B>::zero())
            .expect("Genesis block must be available")
    }
}

impl<B, C> OrderedDataInterpreter<B, C>
where
    B: BlockT,
    B::Header: HeaderT<Number = BlockNumber>,
    C: HeaderBackend<B>,
{
    pub fn new(
        blocks_to_finalize_tx: mpsc::UnboundedSender<BlockHashNum<B>>,
        client: Arc<C>,
        session_boundaries: SessionBoundaries,
    ) -> Self {
        let last_finalized_by_aleph =
            get_last_block_prev_session(session_boundaries.clone(), client.clone());
        let chain_info_provider =
            AuxFinalizationChainInfoProvider::new(client, last_finalized_by_aleph.clone());
        let chain_info_provider =
            CachedChainInfoProvider::new(chain_info_provider, Default::default());

        OrderedDataInterpreter {
            blocks_to_finalize_tx,
            chain_info_provider,
            last_finalized_by_aleph,
            session_boundaries,
        }
    }

    pub fn set_last_finalized(&mut self, block: BlockHashNum<B>) {
        self.last_finalized_by_aleph = block;
    }

    pub fn chain_info_provider(&mut self) -> &mut InterpretersChainInfoProvider<B, C> {
        &mut self.chain_info_provider
    }

    pub fn send_block_to_finalize(
        &mut self,
        block: BlockHashNum<B>,
    ) -> Result<(), TrySendError<BlockHashNum<B>>> {
        self.blocks_to_finalize_tx.unbounded_send(block)
    }

    pub fn blocks_to_finalize_from_data(&mut self, new_data: AlephData<B>) -> Vec<BlockHashNum<B>> {
        let unvalidated_proposal = new_data.head_proposal;
        let proposal = match unvalidated_proposal.validate_bounds(&self.session_boundaries) {
            Ok(proposal) => proposal,
            Err(error) => {
                warn!(target: "aleph-finality", "Incorrect proposal {:?} passed through data availability, session bounds: {:?}, error: {:?}", unvalidated_proposal, self.session_boundaries, error);
                return Vec::new();
            }
        };

        // WARNING: If we ever enable block pruning, this code (and the code in Data Store) must be carefully
        // analyzed for possible safety violations.

        use crate::data_io::proposal::ProposalStatus::*;
        let status = get_proposal_status(&mut self.chain_info_provider, &proposal, None);
        match status {
            Finalize(blocks) => blocks,
            Ignore => {
                debug!(target: "aleph-finality", "Ignoring proposal {:?} in interpreter.", proposal);
                Vec::new()
            }
            Pending(pending_status) => {
                panic!(
                    "Pending proposal {:?} with status {:?} encountered in Data.",
                    proposal, pending_status
                );
            }
        }
    }

    pub fn data_finalized(&mut self, data: AlephData<B>) {
        for block in self.blocks_to_finalize_from_data(data) {
            self.set_last_finalized(block.clone());
            self.chain_info_provider()
                .inner()
                .update_aux_finalized(block.clone());
            if let Err(err) = self.send_block_to_finalize(block) {
                error!(target: "aleph-finality", "Error in sending a block from FinalizationHandler, {}", err);
            }
        }
    }
}
