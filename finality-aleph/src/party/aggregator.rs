use crate::{
    aggregator::BlockSignatureAggregator,
    crypto::KeyBox,
    justification::{AlephJustification, JustificationNotification},
    metrics::Checkpoint,
    network::{DataNetwork, RmcNetworkData},
    party::{AuthoritySubtaskCommon, Task},
    BlockHashNum, Metrics, SessionBoundaries,
};
use aleph_bft::SpawnHandle;
use futures::{
    channel::{mpsc, oneshot},
    StreamExt,
};
use log::{debug, error, trace};
use sc_client_api::HeaderBackend;
use sp_runtime::traits::{Block, Header};
use std::sync::Arc;

/// IO channels used by the aggregator task.
pub struct IO<B: Block> {
    pub blocks_from_interpreter: mpsc::UnboundedReceiver<BlockHashNum<B>>,
    pub justifications_for_chain: mpsc::UnboundedSender<JustificationNotification<B>>,
}

async fn run_aggregator<B, C, N>(
    mut aggregator: BlockSignatureAggregator<'_, B, N, KeyBox>,
    io: IO<B>,
    client: Arc<C>,
    session_boundaries: SessionBoundaries<B>,
    metrics: Option<Metrics<<B::Header as Header>::Hash>>,
    mut exit_rx: oneshot::Receiver<()>,
) where
    B: Block,
    C: HeaderBackend<B> + Send + Sync + 'static,
    N: DataNetwork<RmcNetworkData<B>>,
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

                    trace!(target: "aleph-party", "Received block {:?} in aggregator.", block);
                    if let Some(metrics) = &metrics {
                        metrics.report_block(block.hash, std::time::Instant::now(), Checkpoint::Ordered);
                    }

                    aggregator.start_aggregation(block.hash).await;
                    if block.num == session_boundaries.last_block() {
                        aggregator.notify_last_hash();
                    }
                } else {
                    debug!(target: "aleph-party", "Blocks ended in aggregator. Terminating.");
                    break;
                }
            }
            multisigned_hash = aggregator.next_multisigned_hash() => {
                if let Some((hash, multisignature)) = multisigned_hash {
                    let number = client.number(hash).unwrap().unwrap();
                    // The unwrap might actually fail if data availability is not implemented correctly.
                    let notification = JustificationNotification {
                        justification: AlephJustification{signature: multisignature},
                        hash,
                        number
                    };
                    if let Err(e) = justifications_for_chain.unbounded_send(notification)  {
                        error!(target: "aleph-party", "Issue with sending justification from Aggregator to JustificationHandler {:?}.", e);
                    }
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
    multikeychain: KeyBox,
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
            let aggregator =
                BlockSignatureAggregator::new(rmc_network, &multikeychain, metrics.clone());
            debug!(target: "aleph-party", "Running the aggregator task for {:?}", session_id);
            run_aggregator(aggregator, io, client, session_boundaries, metrics, exit).await;
            debug!(target: "aleph-party", "Aggregator task stopped for {:?}", session_id);
        }
    };

    let handle = spawn_handle.spawn_essential("aleph/consensus_session_aggregator", task);
    Task::new(handle, stop)
}
