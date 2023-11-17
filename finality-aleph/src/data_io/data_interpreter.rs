use std::{default::Default, marker::PhantomData};

use futures::channel::mpsc;
use log::{debug, error, warn};

use crate::{
    block::{Header, HeaderVerifier},
    data_io::{
        chain_info::{AuxFinalizationChainInfoProvider, CachedChainInfoProvider},
        proposal::ProposalStatus,
        status_provider::get_proposal_status,
        AlephData, ChainInfoProvider,
    },
    mpsc::TrySendError,
    BlockId, SessionBoundaries,
};

type InterpretersChainInfoProvider<CIP> =
    CachedChainInfoProvider<AuxFinalizationChainInfoProvider<CIP>>;

/// Takes as input ordered `AlephData` from `AlephBFT` and pushes blocks that should be finalized
/// to an output channel. The other end of the channel is held by the aggregator whose goal is to
/// create multisignatures under the finalized blocks.
pub struct OrderedDataInterpreter<CIP, H, V>
where
    CIP: ChainInfoProvider,
    H: Header,
    V: HeaderVerifier<H>,
{
    blocks_to_finalize_tx: mpsc::UnboundedSender<BlockId>,
    chain_info_provider: InterpretersChainInfoProvider<CIP>,
    verifier: V,
    last_finalized_by_aleph: BlockId,
    session_boundaries: SessionBoundaries,
    _phantom: PhantomData<H>,
}

fn get_last_block_prev_session<CIP>(
    session_boundaries: SessionBoundaries,
    chain_info: &mut CIP,
) -> BlockId
where
    CIP: ChainInfoProvider,
{
    if session_boundaries.first_block() > 0 {
        // We are in session > 0, we take the last block of previous session.
        let last_prev_session_num = session_boundaries.first_block() - 1;
        chain_info.get_finalized_at(last_prev_session_num).expect(
            "Last block of previous session must have been finalized before starting the current",
        )
    } else {
        // We are in session 0, we take the genesis block -- it is finalized by definition.
        chain_info
            .get_finalized_at(0)
            .expect("Genesis block must be available")
    }
}

impl<CIP, H, V> OrderedDataInterpreter<CIP, H, V>
where
    CIP: ChainInfoProvider,
    H: Header,
    V: HeaderVerifier<H>,
{
    pub fn new(
        blocks_to_finalize_tx: mpsc::UnboundedSender<BlockId>,
        mut chain_info: CIP,
        verifier: V,
        session_boundaries: SessionBoundaries,
    ) -> Self {
        let last_finalized_by_aleph =
            get_last_block_prev_session(session_boundaries.clone(), &mut chain_info);
        let chain_info_provider =
            AuxFinalizationChainInfoProvider::new(chain_info, last_finalized_by_aleph.clone());
        let chain_info_provider =
            CachedChainInfoProvider::new(chain_info_provider, Default::default());

        OrderedDataInterpreter {
            blocks_to_finalize_tx,
            chain_info_provider,
            last_finalized_by_aleph,
            session_boundaries,
            verifier,
            _phantom: PhantomData,
        }
    }

    pub fn set_last_finalized(&mut self, block: BlockId) {
        self.last_finalized_by_aleph = block;
    }

    pub fn chain_info_provider(&mut self) -> &mut InterpretersChainInfoProvider<CIP> {
        &mut self.chain_info_provider
    }

    pub fn send_block_to_finalize(&mut self, block: BlockId) -> Result<(), TrySendError<BlockId>> {
        self.blocks_to_finalize_tx.unbounded_send(block)
    }

    pub fn blocks_to_finalize_from_data(
        &mut self,
        new_data: AlephData<H::Unverified>,
    ) -> Vec<BlockId> {
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

        use ProposalStatus::*;
        let status = get_proposal_status(
            &mut self.chain_info_provider,
            &mut self.verifier,
            &proposal,
            None,
        );
        match status {
            Finalize(blocks) => blocks,
            Ignore => {
                debug!(target: "aleph-finality", "Ignoring proposal {:?} in interpreter.", proposal);
                Vec::new()
            }
            Pending(pending_status) => {
                panic!(
                    "Pending proposal {proposal:?} with status {pending_status:?} encountered in Data."
                );
            }
        }
    }

    pub fn data_finalized(&mut self, data: AlephData<H::Unverified>) {
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
