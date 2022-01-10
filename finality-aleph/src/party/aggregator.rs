use crate::{
    aggregator::BlockSignatureAggregator,
    crypto::KeyBox,
    data_io::AlephDataFor,
    finalization::should_finalize,
    justification::{AlephJustification, JustificationNotification},
    metrics::Checkpoint,
    network::RmcNetwork,
    party::{AuthoritySubtaskCommon, Task},
    Metrics,
};
use aleph_bft::SpawnHandle;
use futures::{
    channel::{mpsc, oneshot},
    StreamExt,
};
use log::{debug, error, trace};
use sc_client_api::Backend;
use sp_api::NumberFor;
use sp_runtime::traits::{Block, Header};
use std::sync::Arc;

/// IO channels used by the aggregator task.
pub struct IO<B: Block> {
    pub ordered_units_from_aleph: mpsc::UnboundedReceiver<AlephDataFor<B>>,
    pub justifications_for_chain: mpsc::UnboundedSender<JustificationNotification<B>>,
}

async fn run_aggregator<B, C, BE>(
    mut aggregator: BlockSignatureAggregator<'_, B, KeyBox>,
    io: IO<B>,
    client: Arc<C>,
    last_block_in_session: NumberFor<B>,
    metrics: Option<Metrics<<B::Header as Header>::Hash>>,
    mut exit_rx: oneshot::Receiver<()>,
) where
    B: Block,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B>,
    BE: Backend<B> + 'static,
{
    let IO {
        mut ordered_units_from_aleph,
        justifications_for_chain,
    } = io;
    let mut last_finalized = client.info().finalized_hash;
    let mut last_block_seen = false;
    loop {
        tokio::select! {
            maybe_unit = ordered_units_from_aleph.next() => {
                if let Some(new_block_data) = maybe_unit {
                    trace!(target: "aleph-party", "Received unit {:?} in aggregator.", new_block_data);
                    if last_block_seen {
                        //This is only for optimization purposes.
                        continue;
                    }
                    if let Some(metrics) = &metrics {
                        metrics.report_block(new_block_data.hash, std::time::Instant::now(), Checkpoint::Ordered);
                    }
                    if let Some(data) = should_finalize(last_finalized, new_block_data, client.as_ref(), last_block_in_session) {
                        aggregator.start_aggregation(data.hash).await;
                        last_finalized = data.hash;
                        if data.number == last_block_in_session {
                            aggregator.notify_last_hash();
                            last_block_seen = true;
                        }
                    }
                } else {
                    debug!(target: "aleph-party", "Units ended in aggregator. Terminating.");
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
                    break;
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
pub fn task<B, C, BE>(
    subtask_common: AuthoritySubtaskCommon,
    client: Arc<C>,
    io: IO<B>,
    last_block: NumberFor<B>,
    metrics: Option<Metrics<<B::Header as Header>::Hash>>,
    multikeychain: KeyBox,
    rmc_network: RmcNetwork<B>,
) -> Task
where
    B: Block,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B>,
    BE: Backend<B> + 'static,
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
            run_aggregator(aggregator, io, client, last_block, metrics, exit).await;
            debug!(target: "aleph-party", "Aggregator task stopped for {:?}", session_id);
        }
    };

    let handle = spawn_handle.spawn_essential("aleph/consensus_session_aggregator", task);
    Task::new(handle, stop)
}
