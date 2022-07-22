use std::sync::Arc;

use aleph_bft::{Keychain as BftKeychain, SignatureSet, SpawnHandle};
use aleph_bft_rmc::{DoublingDelayScheduler, ReliableMulticast};
use futures::{
    channel::{mpsc, oneshot},
    StreamExt,
};
use log::{debug, error, trace};
use sc_client_api::HeaderBackend;
use sp_runtime::traits::{Block, Header};

use crate::{
    aggregation::{BlockSignatureAggregator, RmcNetworkData, SignableHash, IO as AggregatorIO},
    crypto::{Keychain, Signature},
    justification::{AlephJustification, JustificationNotification},
    metrics::Checkpoint,
    network::DataNetwork,
    party::{AuthoritySubtaskCommon, Task},
    BlockHashNum, Metrics, SessionBoundaries,
};

/// IO channels used by the aggregator task.
pub struct IO<B: Block> {
    pub blocks_from_interpreter: mpsc::UnboundedReceiver<BlockHashNum<B>>,
    pub justifications_for_chain: mpsc::UnboundedSender<JustificationNotification<B>>,
}

type SignableBlockHash<B> = SignableHash<<B as Block>::Hash>;
type Rmc<'a, B> = ReliableMulticast<'a, SignableBlockHash<B>, Keychain>;

async fn process_new_block_data<B, N>(
    aggregator: &mut AggregatorIO<
        B::Hash,
        RmcNetworkData<B>,
        N,
        SignatureSet<Signature>,
        Rmc<'_, B>,
    >,
    block: BlockHashNum<B>,
    session_boundaries: &SessionBoundaries<B>,
    metrics: &Option<Metrics<<B::Header as Header>::Hash>>,
) where
    B: Block,
    N: DataNetwork<RmcNetworkData<B>>,
    <B as Block>::Hash: AsRef<[u8]>,
{
    trace!(target: "aleph-party", "Received unit {:?} in aggregator.", block);
    if let Some(metrics) = &metrics {
        metrics.report_block(block.hash, std::time::Instant::now(), Checkpoint::Ordered);
    }

    aggregator.start_aggregation(block.hash).await;
    if block.num == session_boundaries.last_block() {
        aggregator.notify_last_hash();
    }
}

fn process_hash<B, C>(
    hash: B::Hash,
    multisignature: SignatureSet<Signature>,
    justifications_for_chain: &mpsc::UnboundedSender<JustificationNotification<B>>,
    client: &Arc<C>,
) where
    B: Block,
    C: HeaderBackend<B> + Send + Sync + 'static,
{
    let number = client.number(hash).unwrap().unwrap();
    // The unwrap might actually fail if data availability is not implemented correctly.
    let notification = JustificationNotification {
        justification: AlephJustification::CommitteeMultisignature(multisignature),
        hash,
        number,
    };
    if let Err(e) = justifications_for_chain.unbounded_send(notification) {
        error!(target: "aleph-party", "Issue with sending justification from Aggregator to JustificationHandler {:?}.", e);
    }
}

async fn run_aggregator<B, C, N>(
    mut aggregator: AggregatorIO<
        B::Hash,
        RmcNetworkData<B>,
        N,
        SignatureSet<Signature>,
        Rmc<'_, B>,
    >,
    io: IO<B>,
    client: Arc<C>,
    session_boundaries: &SessionBoundaries<B>,
    metrics: Option<Metrics<<B::Header as Header>::Hash>>,
    mut exit_rx: oneshot::Receiver<()>,
) where
    B: Block,
    C: HeaderBackend<B> + Send + Sync + 'static,
    N: DataNetwork<RmcNetworkData<B>>,
    <B as Block>::Hash: AsRef<[u8]>,
{
    let IO {
        mut blocks_from_interpreter,
        justifications_for_chain,
    } = io;
    loop {
        trace!(target: "aleph-party", "Aggregator Loop started a next iteration");
        tokio::select! {
            maybe_block = blocks_from_interpreter.next() => {
                if let Some(block) = maybe_block {
                    process_new_block_data(
                        &mut aggregator,
                        block,
                        session_boundaries,
                        &metrics
                    ).await;
                } else {
                    debug!(target: "aleph-party", "Blocks ended in aggregator. Terminating.");
                    break;
                }
            }
            multisigned_hash = aggregator.next_multisigned_hash() => {
                if let Some((hash, multisignature)) = multisigned_hash {
                    process_hash(hash, multisignature, &justifications_for_chain, &client);
                } else {
                    debug!(target: "aleph-party", "The stream of multisigned hashes has ended. Terminating.");
                    return;
                }
            }
            _ = &mut exit_rx => {
                debug!(target: "aleph-party", "Aggregator received exit signal. Terminating.");
                return;
            }
        }
    }
    debug!(target: "aleph-party", "Aggregator awaiting an exit signal.");
    // this allows aggregator to exit after member,
    // otherwise it can exit too early and member complains about a channel to aggregator being closed
    let _ = exit_rx.await;
}

/// Runs the justification signature aggregator within a single session.
pub fn task<B, C, N>(
    subtask_common: AuthoritySubtaskCommon,
    client: Arc<C>,
    io: IO<B>,
    session_boundaries: SessionBoundaries<B>,
    metrics: Option<Metrics<<B::Header as Header>::Hash>>,
    multikeychain: Keychain,
    rmc_network: N,
) -> Task
where
    B: Block,
    C: HeaderBackend<B> + Send + Sync + 'static,
    N: DataNetwork<RmcNetworkData<B>> + 'static,
{
    let AuthoritySubtaskCommon {
        spawn_handle,
        session_id,
    } = subtask_common;
    let (stop, exit) = oneshot::channel();
    let task = {
        async move {
            let (messages_for_rmc, messages_from_network) = mpsc::unbounded();
            let (messages_for_network, messages_from_rmc) = mpsc::unbounded();
            let scheduler = DoublingDelayScheduler::new(tokio::time::Duration::from_millis(500));
            let rmc = ReliableMulticast::new(
                messages_from_network,
                messages_for_network,
                &multikeychain,
                multikeychain.node_count(),
                scheduler,
            );
            let aggregator = BlockSignatureAggregator::new(metrics.clone());
            let aggregator_io = AggregatorIO::new(
                messages_for_rmc,
                messages_from_rmc,
                rmc_network,
                rmc,
                aggregator,
            );
            debug!(target: "aleph-party", "Running the aggregator task for {:?}", session_id);
            run_aggregator(
                aggregator_io,
                io,
                client,
                &session_boundaries,
                metrics,
                exit,
            )
            .await;
            debug!(target: "aleph-party", "Aggregator task stopped for {:?}", session_id);
        }
    };

    let handle = spawn_handle.spawn_essential("aleph/consensus_session_aggregator", task);
    Task::new(handle, stop)
}
