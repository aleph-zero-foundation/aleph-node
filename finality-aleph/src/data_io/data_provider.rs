use crate::{
    data_io::proposal::{UnvalidatedAlephProposal, MAX_DATA_BRANCH_LEN},
    data_io::AlephData,
    metrics::Checkpoint,
    BlockHashNum, Metrics, SessionBoundaries,
};
use async_trait::async_trait;
use futures::channel::oneshot;
use log::{debug, warn};
use sc_client_api::HeaderBackend;
use sp_consensus::SelectChain;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT, NumberFor, One, Zero};
use sp_runtime::{generic::BlockId, SaturatedConversion};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

// Reduce block header to the level given by num, by traversing down via parents.
pub fn reduce_header_to_num<B, C>(client: &C, header: B::Header, num: NumberFor<B>) -> B::Header
where
    B: BlockT,
    C: HeaderBackend<B>,
{
    assert!(
        header.number() >= &num,
        "Cannot reduce {:?} to number {:?}",
        header,
        num
    );
    let mut curr_header = header;
    while curr_header.number() > &num {
        curr_header = client
            .header(BlockId::Hash(*curr_header.parent_hash()))
            .expect("client must respond")
            .expect("parent hash is known by the client");
    }
    curr_header
}

pub fn get_parent<B, C>(client: &C, block: &BlockHashNum<B>) -> Option<BlockHashNum<B>>
where
    B: BlockT,
    C: HeaderBackend<B>,
{
    if block.num.is_zero() {
        return None;
    }
    if let Some(header) = client
        .header(BlockId::Hash(block.hash))
        .expect("client must respond")
    {
        Some((*header.parent_hash(), block.num - <NumberFor<B>>::one()).into())
    } else {
        warn!(target: "afa", "Trying to fetch the parent of an unknown block {:?}.", block);
        None
    }
}

pub fn get_proposal<B, C>(
    client: &C,
    best_block: BlockHashNum<B>,
    finalized_block: BlockHashNum<B>,
) -> Result<AlephData<B>, ()>
where
    B: BlockT,
    C: HeaderBackend<B>,
{
    let mut curr_block = best_block;
    let mut branch: Vec<B::Hash> = Vec::new();
    while curr_block.num > finalized_block.num {
        if curr_block.num - finalized_block.num
            <= <NumberFor<B>>::saturated_from(MAX_DATA_BRANCH_LEN)
        {
            branch.push(curr_block.hash);
        }
        curr_block = get_parent(client, &curr_block).expect("block of num >= 1 must have a parent")
    }
    if curr_block.hash == finalized_block.hash {
        let num_last = finalized_block.num + <NumberFor<B>>::saturated_from(branch.len());
        // The hashes in `branch` are ordered from top to bottom -- need to reverse.
        branch.reverse();
        Ok(AlephData::HeadProposal(UnvalidatedAlephProposal::<B>::new(
            branch, num_last,
        )))
    } else {
        // By backtracking from the best block we reached a block conflicting with best finalized.
        // This is most likely a bug, or some extremely unlikely synchronization issue of the client.
        warn!(target: "afa", "Error computing proposal. Conflicting blocks: {:?}, finalized {:?}", curr_block, finalized_block);
        Err(())
    }
}

const REFRESH_INTERVAL: u64 = 100;

#[derive(PartialEq, Eq, Clone, Debug)]
struct ChainInfo<B: BlockT> {
    best_block_in_session: BlockHashNum<B>,
    highest_finalized: BlockHashNum<B>,
}

/// ChainTracker keeps track of the best_block in a given session and allows to generate `AlephData`.
/// Internally it frequently updates a `data_to_propose` field that is shared with a `DataProvider`, which
/// in turn is a tiny wrapper around this single shared resource that outputs `data_to_propose` whenever
/// `get_data` is called.
pub struct ChainTracker<B, SC, C>
where
    B: BlockT,
    C: HeaderBackend<B> + 'static,
    SC: SelectChain<B> + 'static,
{
    select_chain: SC,
    client: Arc<C>,
    data_to_propose: Arc<Mutex<AlephData<B>>>,
    session_boundaries: SessionBoundaries<B>,
    prev_chain_info: Option<ChainInfo<B>>,
}

impl<B, SC, C> ChainTracker<B, SC, C>
where
    B: BlockT,
    C: HeaderBackend<B> + 'static,
    SC: SelectChain<B> + 'static,
{
    pub fn new(
        select_chain: SC,
        client: Arc<C>,
        session_boundaries: SessionBoundaries<B>,
        metrics: Option<Metrics<<B::Header as HeaderT>::Hash>>,
    ) -> (Self, impl aleph_bft::DataProvider<AlephData<B>>) {
        let data_to_propose = Arc::new(Mutex::new(AlephData::Empty));
        (
            ChainTracker {
                select_chain,
                client,
                data_to_propose: data_to_propose.clone(),
                session_boundaries,
                prev_chain_info: None,
            },
            DataProvider {
                data_to_propose,
                metrics,
            },
        )
    }

    async fn update_data(&mut self, best_block_in_session: &BlockHashNum<B>) {
        // We use best_block_in_session argument and the highest_finalized block from the client and compute
        // the corresponding `AlephData<B>` in `data_to_propose` for AlephBFT. To not recompute this many
        // times we remember these "inputs" in `prev_chain_info` and upon match we leave the old value
        // of `data_to_propose` unaffected.

        let client_info = self.client.info();
        let finalized_block = (client_info.finalized_hash, client_info.finalized_number).into();

        if let Some(prev) = &self.prev_chain_info {
            if prev.best_block_in_session == *best_block_in_session
                && prev.highest_finalized == finalized_block
            {
                // This is exactly the same state that we processed last time in update_data.
                // No point in recomputing.
                return;
            }
        }
        // Update the info for the next call of update data.
        self.prev_chain_info = Some(ChainInfo {
            best_block_in_session: best_block_in_session.clone(),
            highest_finalized: finalized_block.clone(),
        });

        if best_block_in_session.num == finalized_block.num {
            // We don't have anything to propose, we go ahead with an empty proposal.
            *self.data_to_propose.lock().await = AlephData::Empty;
            return;
        }
        if best_block_in_session.num < finalized_block.num {
            // Because of the client synchronization, in extremely rare cases this could happen.
            warn!(target: "afa", "Error updating data. best_block {:?} is lower than finalized {:?}.", best_block_in_session, finalized_block);
            return;
        }

        if let Ok(proposal) = get_proposal(
            &*self.client,
            best_block_in_session.clone(),
            finalized_block,
        ) {
            *self.data_to_propose.lock().await = proposal;
        }
    }

    pub async fn get_best_header(&self) -> B::Header {
        self.select_chain.best_chain().await.expect("No best chain")
    }

    // Returns the highest ancestor of best_block that fits in session_boundaries (typically the best block itself).
    // In case the best block has number less than the first block of session, returns None.
    async fn get_best_block_in_session(
        &self,
        prev_best_block: Option<BlockHashNum<B>>,
    ) -> Option<BlockHashNum<B>> {
        // We employ an optimization here: once the `best_block_in_session` reaches the height of `last_block`
        // (i.e., highest block in session), and the just queried `best_block` is a `descendant` of `prev_best_block`
        // then we don't need to recompute `best_block_in_session`, as `prev_best_block` is already correct.

        let new_best_header = self.get_best_header().await;
        if new_best_header.number() < &self.session_boundaries.first_block() {
            return None;
        }
        let last_block = self.session_boundaries.last_block();
        let new_best_block: BlockHashNum<B> =
            (new_best_header.hash(), *new_best_header.number()).into();
        if new_best_header.number() <= &last_block {
            Some(new_best_block)
        } else {
            match prev_best_block {
                None => {
                    // This is the the first time we see a block in this session.
                    let reduced_header =
                        reduce_header_to_num(&*self.client, new_best_header, last_block);
                    Some((reduced_header.hash(), *reduced_header.number()).into())
                }
                Some(prev) => {
                    if prev.num < last_block {
                        // The previous best block was below the sessioun boundary, we cannot really optimize
                        // but must compute the new best_block_in_session naively.
                        let reduced_header =
                            reduce_header_to_num(&*self.client, new_best_header, last_block);
                        Some((reduced_header.hash(), *reduced_header.number()).into())
                    } else {
                        // Both `prev_best_block` and thus also `new_best_header` are above (or equal to) `last_block`, we optimize.
                        let reduced_header =
                            reduce_header_to_num(&*self.client, new_best_header.clone(), prev.num);
                        if reduced_header.hash() != prev.hash {
                            // The new_best_block is not a descendant of `prev`, we need to update.
                            // In the opposite case we do nothing, as the `prev` is already correct.
                            let reduced_header =
                                reduce_header_to_num(&*self.client, new_best_header, last_block);
                            Some((reduced_header.hash(), *reduced_header.number()).into())
                        } else {
                            Some(prev)
                        }
                    }
                }
            }
        }
    }

    pub async fn run(mut self, mut exit: oneshot::Receiver<()>) {
        let mut best_block_in_session: Option<BlockHashNum<B>> = None;
        loop {
            let delay = futures_timer::Delay::new(Duration::from_millis(REFRESH_INTERVAL));
            tokio::select! {
                _ = delay => {
                    best_block_in_session = self.get_best_block_in_session(best_block_in_session).await;
                    if let Some(best_block) = &best_block_in_session {
                        self.update_data(best_block).await;
                    }

                }
                _ = &mut exit => {
                    debug!(target: "afa", "Task for refreshing best chain received exit signal. Terminating.");
                    return;
                }
            }
        }
    }
}

/// Provides data to AlephBFT for ordering.
#[derive(Clone)]
struct DataProvider<B: BlockT> {
    data_to_propose: Arc<Mutex<AlephData<B>>>,
    metrics: Option<Metrics<<B::Header as HeaderT>::Hash>>,
}

// Honest nodes propose data in session `k` as follows:
// 1. Let `best_block_in_session` be the highest ancestor of the current local view of `best_block` that
//    belongs to session `k`. So either the global `best_block`, or `best_block` but reduced to the last
//    block of session `k` by traversing parents down.
// 2. If the node does not know of any block in session `k` or if `best_block` is equal to the last finalized block
//    then the node proposes `Empty`, otherwise the node proposes a branch extending from one block above
//    last finalized till `best_block` with the restriction that the branch must be truncated to length
//    at most MAX_DATA_BRANCH_LEN.
#[async_trait]
impl<B: BlockT> aleph_bft::DataProvider<AlephData<B>> for DataProvider<B> {
    async fn get_data(&mut self) -> AlephData<B> {
        let data = (*self.data_to_propose.lock().await).clone();

        if let Some(m) = &self.metrics {
            if let AlephData::HeadProposal(proposal) = &data {
                m.report_block(
                    *proposal.branch.last().unwrap(),
                    std::time::Instant::now(),
                    Checkpoint::Ordering,
                );
            }
        }
        debug!(target: "afa", "Outputting {:?} in get_data", data);
        data
    }
}
