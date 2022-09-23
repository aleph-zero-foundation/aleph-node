use std::sync::Arc;

use aleph_aggregator::{BlockSignatureAggregator, SignableHash, IO as Aggregator};
use aleph_bft::{Keychain as BftKeychain, SignatureSet};
use aleph_bft_rmc::{DoublingDelayScheduler, ReliableMulticast};
use futures::{
    channel::{mpsc, oneshot},
    pin_mut, StreamExt,
};
use log::{debug, error, trace};
use sc_client_api::HeaderBackend;
use sp_runtime::traits::{Block, Header};
use tokio::time;

use crate::{
    aggregation::NetworkWrapper,
    crypto::{Keychain, Signature},
    justification::{AlephJustification, JustificationNotification},
    metrics::Checkpoint,
    network::DataNetwork,
    party::{AuthoritySubtaskCommon, Task},
    BlockHashNum, Metrics, RmcNetworkData, SessionBoundaries, STATUS_REPORT_INTERVAL,
};

/// IO channels used by the aggregator task.
pub struct IO<B: Block> {
    pub blocks_from_interpreter: mpsc::UnboundedReceiver<BlockHashNum<B>>,
    pub justifications_for_chain: mpsc::UnboundedSender<JustificationNotification<B>>,
}

type SignableBlockHash<B> = SignableHash<<B as Block>::Hash>;
type Rmc<'a, B> = ReliableMulticast<'a, SignableBlockHash<B>, Keychain>;
type AggregatorIO<'a, B, N> = Aggregator<
    <B as Block>::Hash,
    RmcNetworkData<B>,
    NetworkWrapper<RmcNetworkData<B>, N>,
    SignatureSet<Signature>,
    Rmc<'a, B>,
    Metrics<<B as Block>::Hash>,
>;

async fn process_new_block_data<B, N>(
    aggregator: &mut AggregatorIO<'_, B, N>,
    block: BlockHashNum<B>,
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
}

fn process_hash<B, C>(
    hash: B::Hash,
    multisignature: SignatureSet<Signature>,
    justifications_for_chain: &mpsc::UnboundedSender<JustificationNotification<B>>,
    client: &Arc<C>,
) -> Result<(), ()>
where
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
        return Err(());
    }
    Ok(())
}

async fn run_aggregator<B, C, N>(
    mut aggregator: AggregatorIO<'_, B, N>,
    io: IO<B>,
    client: Arc<C>,
    session_boundaries: &SessionBoundaries<B>,
    metrics: Option<Metrics<<B::Header as Header>::Hash>>,
    mut exit_rx: oneshot::Receiver<()>,
) -> Result<(), ()>
where
    B: Block,
    C: HeaderBackend<B> + Send + Sync + 'static,
    N: DataNetwork<RmcNetworkData<B>>,
    <B as Block>::Hash: AsRef<[u8]>,
{
    let IO {
        blocks_from_interpreter,
        justifications_for_chain,
    } = io;

    let blocks_from_interpreter = blocks_from_interpreter.take_while(|block| {
        let block_num = block.num;
        async move {
            if block_num == session_boundaries.last_block() {
                debug!(target: "aleph-party", "Aggregator is processing last block in session.");
            }
            block_num <= session_boundaries.last_block()
        }
    });
    pin_mut!(blocks_from_interpreter);
    let mut hash_of_last_block = None;
    let mut no_more_blocks = false;

    let mut status_ticker = time::interval(STATUS_REPORT_INTERVAL);

    loop {
        trace!(target: "aleph-party", "Aggregator Loop started a next iteration");
        tokio::select! {
            maybe_block = blocks_from_interpreter.next() => {
                if let Some(block) = maybe_block {
                    hash_of_last_block = Some(block.hash);
                    process_new_block_data::<B, N>(
                        &mut aggregator,
                        block,
                        &metrics
                    ).await;
                } else {
                    debug!(target: "aleph-party", "Blocks ended in aggregator.");
                    no_more_blocks = true;
                }
            }
            multisigned_hash = aggregator.next_multisigned_hash() => {
                if let Some((hash, multisignature)) = multisigned_hash {
                    process_hash(hash, multisignature, &justifications_for_chain, &client)?;
                    if Some(hash) == hash_of_last_block {
                        hash_of_last_block = None;
                    }
                } else {
                    debug!(target: "aleph-party", "The stream of multisigned hashes has ended. Terminating.");
                    break;
                }
            }
            _ = status_ticker.tick() => {
                aggregator.status_report();
            },
            _ = &mut exit_rx => {
                debug!(target: "aleph-party", "Aggregator received exit signal. Terminating.");
                break;
            }
        }
        if hash_of_last_block.is_none() && no_more_blocks {
            debug!(target: "aleph-party", "Aggregator processed all provided blocks. Terminating.");
            break;
        }
    }
    debug!(target: "aleph-party", "Aggregator finished its work.");
    Ok(())
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
            let aggregator_io = AggregatorIO::<B, N>::new(
                messages_for_rmc,
                messages_from_rmc,
                NetworkWrapper::new(rmc_network),
                rmc,
                aggregator,
            );
            debug!(target: "aleph-party", "Running the aggregator task for {:?}", session_id);
            let result = run_aggregator(
                aggregator_io,
                io,
                client,
                &session_boundaries,
                metrics,
                exit,
            )
            .await;
            debug!(target: "aleph-party", "Aggregator task stopped for {:?}", session_id);
            result
        }
    };

    let handle =
        spawn_handle.spawn_essential_with_result("aleph/consensus_session_aggregator", task);
    Task::new(handle, stop)
}
