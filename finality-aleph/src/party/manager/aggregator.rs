use std::sync::Arc;

use aleph_primitives::BlockNumber;
use futures::{
    channel::{mpsc, oneshot},
    pin_mut, StreamExt,
};
use log::{debug, error, trace};
use sc_client_api::HeaderBackend;
use sp_runtime::traits::{Block, Header};
use tokio::time;

use crate::{
    abft::SignatureSet,
    aggregation::Aggregator,
    crypto::Signature,
    justification::AlephJustification,
    metrics::Checkpoint,
    network::data::Network,
    party::{
        manager::aggregator::AggregatorVersion::{Current, Legacy},
        AuthoritySubtaskCommon, Task,
    },
    sync::{substrate::Justification, JustificationSubmissions, JustificationTranslator},
    BlockId, CurrentRmcNetworkData, IdentifierFor, Keychain, LegacyRmcNetworkData, Metrics,
    SessionBoundaries, STATUS_REPORT_INTERVAL,
};

/// IO channels used by the aggregator task.
pub struct IO<H, JS, JT>
where
    H: Header<Number = BlockNumber>,
    JS: JustificationSubmissions<Justification<H>> + Send + Sync + Clone,
    JT: JustificationTranslator<H> + Send + Sync + Clone,
{
    pub blocks_from_interpreter: mpsc::UnboundedReceiver<BlockId<H>>,
    pub justifications_for_chain: JS,
    pub justification_translator: JT,
}

async fn process_new_block_data<B, CN, LN>(
    aggregator: &mut Aggregator<'_, B, CN, LN>,
    block: IdentifierFor<B>,
    metrics: &Metrics<<B::Header as Header>::Hash>,
) where
    B: Block,
    B::Header: Header<Number = BlockNumber>,
    CN: Network<CurrentRmcNetworkData<B>>,
    LN: Network<LegacyRmcNetworkData<B>>,
    <B as Block>::Hash: AsRef<[u8]>,
{
    trace!(target: "aleph-party", "Received unit {:?} in aggregator.", block);
    metrics.report_block(block.hash, std::time::Instant::now(), Checkpoint::Ordered);

    aggregator.start_aggregation(block.hash).await;
}

fn process_hash<B, C, JS, JT>(
    hash: B::Hash,
    multisignature: SignatureSet<Signature>,
    justifications_for_chain: &mut JS,
    justification_translator: &JT,
    client: &Arc<C>,
) -> Result<(), ()>
where
    B: Block,
    B::Header: Header<Number = BlockNumber>,
    C: HeaderBackend<B> + Send + Sync + 'static,
    JS: JustificationSubmissions<Justification<B::Header>> + Send + Sync + Clone,
    JT: JustificationTranslator<B::Header> + Send + Sync + Clone,
{
    let number = client.number(hash).unwrap().unwrap();
    // The unwrap might actually fail if data availability is not implemented correctly.
    let justification = match justification_translator.translate(
        AlephJustification::CommitteeMultisignature(multisignature),
        BlockId::new(hash, number),
    ) {
        Ok(justification) => justification,
        Err(e) => {
            error!(target: "aleph-party", "Issue with translating justification from Aggregator to Sync Justification: {}.", e);
            return Err(());
        }
    };
    if let Err(e) = justifications_for_chain.submit(justification) {
        error!(target: "aleph-party", "Issue with sending justification from Aggregator to JustificationHandler {}.", e);
        return Err(());
    }
    Ok(())
}

async fn run_aggregator<B, C, CN, LN, JS, JT>(
    mut aggregator: Aggregator<'_, B, CN, LN>,
    io: IO<B::Header, JS, JT>,
    client: Arc<C>,
    session_boundaries: &SessionBoundaries,
    metrics: Metrics<<B::Header as Header>::Hash>,
    mut exit_rx: oneshot::Receiver<()>,
) -> Result<(), ()>
where
    B: Block,
    B::Header: Header<Number = BlockNumber>,
    JS: JustificationSubmissions<Justification<B::Header>> + Send + Sync + Clone,
    JT: JustificationTranslator<B::Header> + Send + Sync + Clone,
    C: HeaderBackend<B> + Send + Sync + 'static,
    LN: Network<LegacyRmcNetworkData<B>>,
    CN: Network<CurrentRmcNetworkData<B>>,
    <B as Block>::Hash: AsRef<[u8]>,
{
    let IO {
        blocks_from_interpreter,
        mut justifications_for_chain,
        justification_translator,
    } = io;

    let blocks_from_interpreter = blocks_from_interpreter.take_while(|block| {
        let block_num = block.number;
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
                    process_new_block_data::<B, CN, LN>(
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
                    process_hash(hash, multisignature, &mut justifications_for_chain, &justification_translator, &client)?;
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

pub enum AggregatorVersion<CN, LN> {
    Current(CN),
    Legacy(LN),
}

/// Runs the justification signature aggregator within a single session.
pub fn task<B, C, CN, LN, JS, JT>(
    subtask_common: AuthoritySubtaskCommon,
    client: Arc<C>,
    io: IO<B::Header, JS, JT>,
    session_boundaries: SessionBoundaries,
    metrics: Metrics<<B::Header as Header>::Hash>,
    multikeychain: Keychain,
    version: AggregatorVersion<CN, LN>,
) -> Task
where
    B: Block,
    B::Header: Header<Number = BlockNumber>,
    JS: JustificationSubmissions<Justification<B::Header>> + Send + Sync + Clone + 'static,
    JT: JustificationTranslator<B::Header> + Send + Sync + Clone + 'static,
    C: HeaderBackend<B> + Send + Sync + 'static,
    LN: Network<LegacyRmcNetworkData<B>> + 'static,
    CN: Network<CurrentRmcNetworkData<B>> + 'static,
{
    let AuthoritySubtaskCommon {
        spawn_handle,
        session_id,
    } = subtask_common;
    let (stop, exit) = oneshot::channel();
    let task = {
        async move {
            let aggregator_io = match version {
                Current(rmc_network) => {
                    Aggregator::new_current(&multikeychain, rmc_network, metrics.clone())
                }
                Legacy(rmc_network) => {
                    Aggregator::new_legacy(&multikeychain, rmc_network, metrics.clone())
                }
            };
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
