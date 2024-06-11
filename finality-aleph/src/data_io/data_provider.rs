use std::{marker::PhantomData, sync::Arc, time::Duration};

use futures::channel::oneshot;
use log::{debug, error, warn};
use parking_lot::Mutex;
use sp_runtime::{traits::Zero, SaturatedConversion};

use crate::{
    aleph_primitives::BlockNumber,
    block::{BestBlockSelector, Header, HeaderBackend, UnverifiedHeader},
    data_io::{proposal::UnvalidatedAlephProposal, AlephData, MAX_DATA_BRANCH_LEN},
    metrics::{Checkpoint, TimingBlockMetrics},
    party::manager::Runnable,
    BlockId, SessionBoundaries,
};

const LOG_TARGET: &str = "aleph-data-store";

// Reduce block header to the level given by num, by traversing down via parents.
pub fn reduce_header_to_num<H, C>(client: &C, header: H, num: BlockNumber) -> H
where
    H: Header,
    C: HeaderBackend<H>,
{
    assert!(
        header.id().number() >= num,
        "Cannot reduce {header:?} to number {num:?}"
    );
    let mut curr_header = header;
    while curr_header.id().number() > num {
        curr_header = client
            .header(
                &curr_header
                    .parent_id()
                    .expect("number() > num >= 0, so parent exists qed."),
            )
            .expect("client must respond")
            .expect("parent hash is known by the client");
    }
    curr_header
}

pub fn get_parent<H, C>(client: &C, block: &BlockId) -> Option<BlockId>
where
    H: Header,
    C: HeaderBackend<H>,
{
    if block.number().is_zero() {
        return None;
    }
    if let Some(header) = client.header(block).expect("client must respond") {
        Some(header.parent_id()?)
    } else {
        warn!(
            target: LOG_TARGET,
            "Trying to fetch the parent of an unknown block {:?}.", block
        );
        None
    }
}

pub enum ProposalPreparationError {
    MissingHeader,
    BestContradictsFinalized,
}

pub fn get_proposal<H, C>(
    client: &C,
    best_block: BlockId,
    finalized_block: BlockId,
) -> Result<Option<AlephData<H::Unverified>>, ProposalPreparationError>
where
    H: Header,
    C: HeaderBackend<H>,
{
    use ProposalPreparationError::*;
    let mut curr_block = best_block;
    let mut branch = Vec::new();
    while curr_block.number() > finalized_block.number() {
        if curr_block.number() - finalized_block.number()
            <= <BlockNumber>::saturated_from(MAX_DATA_BRANCH_LEN)
        {
            branch.push(curr_block.clone());
        }
        curr_block = get_parent(client, &curr_block).expect("block of num >= 1 must have a parent")
    }
    if curr_block == finalized_block {
        let mut branch = branch.into_iter();
        let head_id = match branch.next() {
            Some(id) => id,
            None => return Ok(None),
        };
        let head = match client.header(&head_id) {
            Ok(Some(header)) => header,
            _ => return Err(MissingHeader),
        };
        let tail: Vec<_> = branch.rev().map(|id| id.hash()).collect();
        Ok(Some(AlephData {
            head_proposal: UnvalidatedAlephProposal::new(head.into_unverified(), tail),
        }))
    } else {
        // By backtracking from the best block we reached a block conflicting with best finalized.
        // This is most likely a bug, or some extremely unlikely synchronization issue of the client.
        warn!(
            target: LOG_TARGET,
            "Error computing proposal. Conflicting blocks: {:?}, finalized {:?}",
            curr_block,
            finalized_block
        );
        Err(BestContradictsFinalized)
    }
}

const DEFAULT_REFRESH_INTERVAL: Duration = Duration::from_millis(100);

pub struct ChainTrackerConfig {
    pub refresh_interval: Duration,
}

impl Default for ChainTrackerConfig {
    fn default() -> ChainTrackerConfig {
        ChainTrackerConfig {
            refresh_interval: DEFAULT_REFRESH_INTERVAL,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
struct ChainInfo {
    best_block_in_session: BlockId,
    highest_finalized: BlockId,
}

/// ChainTracker keeps track of the best_block in a given session and allows to generate `AlephData`.
/// Internally it frequently updates a `data_to_propose` field that is shared with a `DataProvider`, which
/// in turn is a tiny wrapper around this single shared resource that takes out `data_to_propose` whenever
/// `get_data` is called.
pub struct ChainTracker<H, BBS, C>
where
    H: Header,
    C: HeaderBackend<H>,
    BBS: BestBlockSelector<H> + 'static,
{
    chain_tip_selections_strategy: BBS,
    client: C,
    data_to_propose: Arc<Mutex<Option<AlephData<H::Unverified>>>>,
    session_boundaries: SessionBoundaries,
    prev_chain_info: Option<ChainInfo>,
    config: ChainTrackerConfig,
    _phantom: PhantomData<H>,
}

impl<H, BBS, C> ChainTracker<H, BBS, C>
where
    H: Header,
    C: HeaderBackend<H>,
    BBS: BestBlockSelector<H> + 'static,
{
    pub fn new(
        chain_tip_selections_strategy: BBS,
        client: C,
        session_boundaries: SessionBoundaries,
        config: ChainTrackerConfig,
        metrics: TimingBlockMetrics,
    ) -> (Self, DataProvider<H::Unverified>) {
        let data_to_propose = Arc::new(Mutex::new(None));
        (
            ChainTracker {
                chain_tip_selections_strategy,
                client,
                data_to_propose: data_to_propose.clone(),
                session_boundaries,
                prev_chain_info: None,
                config,
                _phantom: PhantomData,
            },
            DataProvider {
                data_to_propose,
                metrics,
            },
        )
    }

    fn update_data(&mut self, best_block_in_session: &BlockId) {
        // We use best_block_in_session argument and the top_finalized block from the client and compute
        // the corresponding `AlephData<B>` in `data_to_propose` for AlephBFT. To not recompute this many
        // times we remember these "inputs" in `prev_chain_info` and upon match we leave the old value
        // of `data_to_propose` unaffected.
        let finalized_block = self.client.top_finalized_id();
        if finalized_block.number() >= self.session_boundaries.last_block() {
            // This session is already finished, but this instance of ChainTracker has not been terminated yet.
            // We go with the default -- empty proposal, this does not have any significance.
            *self.data_to_propose.lock() = None;
            return;
        }

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

        if best_block_in_session.number() < finalized_block.number() {
            // Because of the client synchronization, in extremely rare cases this could happen.
            warn!(
                target: LOG_TARGET,
                "Error updating data. best_block {:?} is lower than finalized {:?}.",
                best_block_in_session,
                finalized_block
            );
            return;
        }

        if let Ok(proposal) =
            get_proposal(&self.client, best_block_in_session.clone(), finalized_block)
        {
            *self.data_to_propose.lock() = proposal;
        }
    }

    async fn get_best_header(&self) -> H {
        self.chain_tip_selections_strategy
            .select_best()
            .await
            .expect("Failed to select the chain tip")
    }

    // Returns the highest ancestor of best_block that fits in session_boundaries (typically the best block itself).
    // In case the best block has number less than the first block of session, returns None.
    async fn get_best_block_in_session(&self, prev_best_block: Option<BlockId>) -> Option<BlockId> {
        // We employ an optimization here: once the `best_block_in_session` reaches the height of `last_block`
        // (i.e., highest block in session), and the just queried `best_block` is a `descendant` of `prev_best_block`
        // then we don't need to recompute `best_block_in_session`, as `prev_best_block` is already correct.

        let new_best_header = self.get_best_header().await;
        if new_best_header.id().number() < self.session_boundaries.first_block() {
            return None;
        }
        let last_block = self.session_boundaries.last_block();
        let new_best_block = new_best_header.id();
        if new_best_header.id().number() <= last_block {
            Some(new_best_block)
        } else {
            match prev_best_block {
                None => {
                    // This is the the first time we see a block in this session.
                    let reduced_header =
                        reduce_header_to_num(&self.client, new_best_header, last_block);
                    Some(reduced_header.id())
                }
                Some(prev) => {
                    if prev.number() < last_block {
                        // The previous best block was below the sessioun boundary, we cannot really optimize
                        // but must compute the new best_block_in_session naively.
                        let reduced_header =
                            reduce_header_to_num(&self.client, new_best_header, last_block);
                        Some(reduced_header.id())
                    } else {
                        // Both `prev_best_block` and thus also `new_best_header` are above (or equal to) `last_block`, we optimize.
                        let reduced_header = reduce_header_to_num(
                            &self.client,
                            new_best_header.clone(),
                            prev.number(),
                        );
                        if reduced_header.id().hash() != prev.hash() {
                            // The new_best_block is not a descendant of `prev`, we need to update.
                            // In the opposite case we do nothing, as the `prev` is already correct.
                            let reduced_header =
                                reduce_header_to_num(&self.client, new_best_header, last_block);
                            Some(reduced_header.id())
                        } else {
                            Some(prev)
                        }
                    }
                }
            }
        }
    }

    async fn run(mut self) {
        let mut best_block_in_session: Option<BlockId> = None;
        loop {
            futures_timer::Delay::new(self.config.refresh_interval).await;
            best_block_in_session = self.get_best_block_in_session(best_block_in_session).await;
            if let Some(best_block) = &best_block_in_session {
                self.update_data(best_block);
            }
        }
    }
}

#[async_trait::async_trait]
impl<H, BBS, C> Runnable for ChainTracker<H, BBS, C>
where
    H: Header,
    C: HeaderBackend<H> + 'static,
    BBS: BestBlockSelector<H> + 'static,
{
    async fn run(mut self, exit: oneshot::Receiver<()>) {
        tokio::select! {
            _ = self.run() => error!(target: LOG_TARGET, "Task for refreshing best chain finished."),
            _ = exit => debug!(target: LOG_TARGET, "Task for refreshing best chain received exit signal. Terminating."),
        }
    }
}

/// Provides data to AlephBFT for ordering.
#[derive(Clone)]
pub struct DataProvider<UH: UnverifiedHeader> {
    data_to_propose: Arc<Mutex<Option<AlephData<UH>>>>,
    metrics: TimingBlockMetrics,
}

// Honest nodes propose data in session `k` as follows:
// 1. Let `best_block_in_session` be the highest ancestor of the current local view of `best_block` that
//    belongs to session `k`. So either the global `best_block`, or `best_block` but reduced to the last
//    block of session `k` by traversing parents down.
// 2. If the node does not know of any block in session `k` or if `best_block` is equal to the last finalized block
//    then the node proposes `Empty`, otherwise the node proposes a branch extending from one block above
//    last finalized till `best_block` with the restriction that the branch must be truncated to length
//    at most MAX_DATA_BRANCH_LEN.
impl<UH: UnverifiedHeader> DataProvider<UH> {
    pub async fn get_data(&mut self) -> Option<AlephData<UH>> {
        let data_to_propose = (*self.data_to_propose.lock()).take();

        if let Some(data) = &data_to_propose {
            let top_block = data.head_proposal.top_block();
            self.metrics
                .report_block(top_block.hash(), Checkpoint::Proposed);
            debug!(target: LOG_TARGET, "Outputting {:?} in get_data", data);
        };

        data_to_propose
    }
}

#[cfg(test)]
mod tests {
    use std::{future::Future, sync::Arc, time::Duration};

    use futures::channel::oneshot;
    use tokio::time::sleep;

    use crate::{
        data_io::{
            data_provider::{ChainTracker, ChainTrackerConfig},
            AlephData, DataProvider, MAX_DATA_BRANCH_LEN,
        },
        metrics::TimingBlockMetrics,
        party::manager::Runnable,
        testing::{
            client_chain_builder::ClientChainBuilder,
            mocks::{aleph_data_from_blocks, THeader, TestClientBuilder, TestClientBuilderExt},
        },
        SessionBoundaryInfo, SessionId, SessionPeriod,
    };

    const SESSION_LEN: u32 = 100;
    // The lower the interval the less time the tests take, however setting this too low might cause
    // the tests to fail.
    const REFRESH_INTERVAL: Duration = Duration::from_millis(5);
    //  Sleep time that's usually enough for the internal refreshing in ChainTracker to finish.
    const SLEEP_TIME: Duration = Duration::from_millis(15);

    fn prepare_chain_tracker_test() -> (
        impl Future<Output = ()>,
        oneshot::Sender<()>,
        ClientChainBuilder,
        DataProvider<THeader>,
    ) {
        let (client, select_chain) = TestClientBuilder::new().build_with_longest_chain();
        let client = Arc::new(client);

        let chain_builder =
            ClientChainBuilder::new(client.clone(), Arc::new(TestClientBuilder::new().build()));
        let session_boundaries = SessionBoundaryInfo::new(SessionPeriod(SESSION_LEN))
            .boundaries_for_session(SessionId(0));

        let config = ChainTrackerConfig {
            refresh_interval: REFRESH_INTERVAL,
        };

        let (chain_tracker, data_provider) = ChainTracker::new(
            select_chain,
            client,
            session_boundaries,
            config,
            TimingBlockMetrics::noop(),
        );

        let (exit_chain_tracker_tx, exit_chain_tracker_rx) = oneshot::channel();
        (
            async move {
                Runnable::run(chain_tracker, exit_chain_tracker_rx).await;
            },
            exit_chain_tracker_tx,
            chain_builder,
            data_provider,
        )
    }

    // Retries sleeping and checking if data in provider is available.
    // This theoretically might fail, but practically shouldn't.
    async fn sleep_until_data_available(
        data_provider: &mut DataProvider<THeader>,
    ) -> AlephData<THeader> {
        const RETRIES: u128 = 1000;
        for _ in 0..RETRIES {
            sleep(SLEEP_TIME).await;
            let maybe_data = data_provider.get_data().await;
            if let Some(data) = maybe_data {
                return data;
            }
        }
        panic!(
            "Data not available after {}ms (usually should be available after {}ms).",
            RETRIES * REFRESH_INTERVAL.as_millis(),
            REFRESH_INTERVAL.as_millis()
        );
    }

    async fn run_test<F, S>(scenario: S)
    where
        F: Future,
        S: FnOnce(ClientChainBuilder, DataProvider<THeader>) -> F,
    {
        let (task_handle, exit, chain_builder, data_provider) = prepare_chain_tracker_test();
        let chain_tracker_handle = tokio::spawn(task_handle);

        scenario(chain_builder, data_provider).await;

        exit.send(()).unwrap();
        chain_tracker_handle
            .await
            .expect("Chain tracker did not terminate cleanly.");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn proposes_empty_and_nonempty_when_expected() {
        run_test(|mut chain_builder, mut data_provider| async move {
            sleep(SLEEP_TIME).await;

            assert_eq!(
                data_provider.get_data().await,
                None,
                "Expected empty proposal"
            );

            let blocks = chain_builder
                .initialize_single_branch_and_import(2 * MAX_DATA_BRANCH_LEN)
                .await;

            let data = sleep_until_data_available(&mut data_provider).await;
            let expected_data = aleph_data_from_blocks(blocks[..MAX_DATA_BRANCH_LEN].to_vec());
            assert_eq!(data, expected_data);
        })
        .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn proposal_changes_with_finalization() {
        run_test(|mut chain_builder, mut data_provider| async move {
            let blocks = chain_builder
                .initialize_single_branch_and_import(3 * MAX_DATA_BRANCH_LEN)
                .await;
            for height in 1..(2 * MAX_DATA_BRANCH_LEN) {
                chain_builder.finalize_block(&blocks[height - 1].header.hash());
                let data = sleep_until_data_available(&mut data_provider).await;
                let expected_data =
                    aleph_data_from_blocks(blocks[height..(MAX_DATA_BRANCH_LEN + height)].to_vec());
                assert_eq!(data, expected_data);
            }
            chain_builder.finalize_block(&blocks.last().unwrap().header.hash());
            sleep(SLEEP_TIME).await;
            assert_eq!(
                data_provider.get_data().await,
                None,
                "Expected empty proposal"
            );
        })
        .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn returns_empty_proposal_above_session_end() {
        run_test(|mut chain_builder, mut data_provider| async move {
            let blocks = chain_builder
                .initialize_single_branch_and_import(
                    (SESSION_LEN as usize) + 3 * MAX_DATA_BRANCH_LEN,
                )
                .await;
            let data = sleep_until_data_available(&mut data_provider).await;
            let expected_data = aleph_data_from_blocks(blocks[0..MAX_DATA_BRANCH_LEN].to_vec());
            assert_eq!(data, expected_data);

            // Finalize a block beyond the last block in the session.
            chain_builder.finalize_block(&blocks.last().unwrap().header.hash());
            sleep(SLEEP_TIME).await;
            assert_eq!(
                data_provider.get_data().await,
                None,
                "Expected empty proposal"
            );
        })
        .await;
    }
}
