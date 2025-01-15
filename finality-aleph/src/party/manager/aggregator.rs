use std::fmt::Display;

use futures::{
    channel::{mpsc, oneshot},
    pin_mut,
    stream::FusedStream,
    StreamExt,
};
use log::{debug, error, trace};
use tokio::time;

use crate::{
    abft::SignatureSet,
    aggregation::{Aggregator, SignableTypedHash},
    aleph_primitives::{
        crypto::SignatureSet as PrimitivesSignatureSet, AuthoritySignature, BlockHash, Hash,
    },
    block::{
        substrate::{Justification, JustificationTranslator},
        Header, HeaderBackend,
    },
    crypto::Signature,
    justification::AlephJustification,
    metrics::{Checkpoint, TimingBlockMetrics},
    network::data::Network,
    party::{
        manager::aggregator::AggregatorVersion::{Current, Legacy},
        AuthoritySubtaskCommon, Task, LOG_TARGET,
    },
    sync::JustificationSubmissions,
    BlockId, CurrentRmcNetworkData, Keychain, LegacyRmcNetworkData, SessionBoundaries,
    STATUS_REPORT_INTERVAL,
};

#[derive(Debug)]
pub enum Error {
    MultisignaturesStreamTerminated,
    UnableToProcessHash,
    UnableToSendSignedPerformance,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;
        match self {
            MultisignaturesStreamTerminated => {
                write!(f, "the stream of multisigned hashes has ended")
            }
            UnableToProcessHash => write!(f, "error while processing a block hash"),
            UnableToSendSignedPerformance => {
                write!(f, "failed to send a signed performance hash to the scorer")
            }
        }
    }
}

/// IO channels used by the aggregator task.
pub struct IO<JS>
where
    JS: JustificationSubmissions<Justification> + Send + Sync + Clone,
{
    pub blocks_from_interpreter: mpsc::UnboundedReceiver<BlockId>,
    pub justifications_for_chain: JS,
    pub justification_translator: JustificationTranslator,
    pub performance_from_scorer: mpsc::UnboundedReceiver<Hash>,
    pub signed_performance_for_scorer:
        mpsc::UnboundedSender<(Hash, PrimitivesSignatureSet<AuthoritySignature>)>,
}

async fn process_new_block_data<CN, LN>(
    aggregator: &mut Aggregator<CN, LN>,
    block: BlockId,
    metrics: &mut TimingBlockMetrics,
) where
    CN: Network<CurrentRmcNetworkData>,
    LN: Network<LegacyRmcNetworkData>,
{
    trace!(target: LOG_TARGET, "Received unit {:?} in aggregator.", block);
    let hash = block.hash();
    metrics.report_block(hash, Checkpoint::Ordered);
    aggregator
        .start_aggregation(SignableTypedHash::Block(hash))
        .await;
}

fn process_block_hash<H, C, JS>(
    hash: BlockHash,
    multisignature: SignatureSet<Signature>,
    justifications_for_chain: &mut JS,
    justification_translator: &JustificationTranslator,
    client: &C,
) -> Result<(), ()>
where
    H: Header,
    C: HeaderBackend<H> + 'static,
    JS: JustificationSubmissions<Justification> + Send + Sync + Clone,
{
    let number = client.hash_to_id(hash).unwrap().unwrap().number();
    // The unwrap might actually fail if data availability is not implemented correctly.
    let justification = match justification_translator.translate(
        AlephJustification::CommitteeMultisignature(multisignature),
        BlockId::new(hash, number),
    ) {
        Ok(justification) => justification,
        Err(e) => {
            error!(target: LOG_TARGET, "Issue with translating justification from Aggregator to Sync Justification: {}.", e);
            return Err(());
        }
    };
    if let Err(e) = justifications_for_chain.submit(justification) {
        error!(target: LOG_TARGET, "Issue with sending justification from Aggregator to JustificationHandler {}.", e);
        return Err(());
    }
    Ok(())
}

async fn run_aggregator<H, C, CN, LN, JS>(
    mut aggregator: Aggregator<CN, LN>,
    io: IO<JS>,
    client: C,
    session_boundaries: &SessionBoundaries,
    mut metrics: TimingBlockMetrics,
    mut exit_rx: oneshot::Receiver<()>,
) -> Result<(), Error>
where
    H: Header,
    JS: JustificationSubmissions<Justification> + Send + Sync + Clone,
    C: HeaderBackend<H> + 'static,
    LN: Network<LegacyRmcNetworkData>,
    CN: Network<CurrentRmcNetworkData>,
{
    use SignableTypedHash::*;
    let IO {
        blocks_from_interpreter,
        mut justifications_for_chain,
        justification_translator,
        performance_from_scorer,
        signed_performance_for_scorer,
    } = io;

    let blocks_from_interpreter = blocks_from_interpreter.take_while(|block| {
        let block_num = block.number();
        async move {
            if block_num == session_boundaries.last_block() {
                debug!(target: LOG_TARGET, "Aggregator is processing last block in session.");
            }
            block_num <= session_boundaries.last_block()
        }
    });
    pin_mut!(blocks_from_interpreter);
    pin_mut!(performance_from_scorer);
    let mut hash_of_last_block = None;
    let mut session_over = blocks_from_interpreter.is_terminated();
    let mut no_more_performance = performance_from_scorer.is_terminated();

    let mut status_ticker = time::interval(STATUS_REPORT_INTERVAL);

    loop {
        trace!(target: LOG_TARGET, "Aggregator Loop started a next iteration");
        tokio::select! {
            maybe_block = blocks_from_interpreter.next(), if !session_over => match maybe_block {
                Some(block) => {
                    hash_of_last_block = Some(block.hash());
                    process_new_block_data::<CN, LN>(
                        &mut aggregator,
                        block,
                        &mut metrics
                    ).await;
                },
                None => {
                    debug!(target: LOG_TARGET, "Blocks ended in aggregator.");
                    session_over = true;
                },
            },
            maybe_performance_hash = performance_from_scorer.next(), if !no_more_performance && !session_over => match maybe_performance_hash {
                Some(hash) => {
                    aggregator
                        .start_aggregation(SignableTypedHash::Performance(hash))
                        .await;
                },
                None => {
                    debug!(target: LOG_TARGET, "Performance hashes ended in aggregator.");
                    no_more_performance = true;
                },
            },
            multisigned_hash = aggregator.next_multisigned_hash() => {
                let (hash, multisignature) = multisigned_hash.ok_or(Error::MultisignaturesStreamTerminated)?;
                match hash {
                    Block(hash) => {
                        process_block_hash(hash, multisignature, &mut justifications_for_chain, &justification_translator, &client).map_err(|_| Error::UnableToProcessHash)?;
                        if Some(hash) == hash_of_last_block {
                            hash_of_last_block = None;
                        }
                    },
                    Performance(hash) => {
                        if let Err(e) = signed_performance_for_scorer.unbounded_send((hash, multisignature.into())) {
                            error!(target: LOG_TARGET, "Issue with sending signed performance hash from Aggregator to Scorer {}.", e);
                            return Err(Error::UnableToSendSignedPerformance);
                        }
                    }
                }
            },
            _ = status_ticker.tick() => {
                aggregator.status_report();
            },
            _ = &mut exit_rx => {
                debug!(target: LOG_TARGET, "Aggregator received exit signal. Terminating.");
                break;
            }
        }
        if hash_of_last_block.is_none() && session_over {
            debug!(target: LOG_TARGET, "Aggregator processed all provided blocks. Terminating.");
            break;
        }
    }
    debug!(target: LOG_TARGET, "Aggregator finished its work.");
    Ok(())
}

pub enum AggregatorVersion<CN, LN> {
    Current(CN),
    Legacy(LN),
}

/// Runs the justification signature aggregator within a single session.
pub fn task<H, C, CN, LN, JS>(
    subtask_common: AuthoritySubtaskCommon,
    client: C,
    io: IO<JS>,
    session_boundaries: SessionBoundaries,
    metrics: TimingBlockMetrics,
    multikeychain: Keychain,
    version: AggregatorVersion<CN, LN>,
) -> Task
where
    H: Header,
    JS: JustificationSubmissions<Justification> + Send + Sync + Clone + 'static,
    C: HeaderBackend<H> + 'static,
    LN: Network<LegacyRmcNetworkData> + 'static,
    CN: Network<CurrentRmcNetworkData> + 'static,
{
    let AuthoritySubtaskCommon {
        spawn_handle,
        session_id,
    } = subtask_common;
    let (stop, exit) = oneshot::channel();
    let task = {
        async move {
            let aggregator_io = match version {
                Current(rmc_network) => Aggregator::new_current(&multikeychain, rmc_network),
                Legacy(rmc_network) => Aggregator::new_legacy(&multikeychain, rmc_network),
            };
            debug!(target: LOG_TARGET, "Running the aggregator task for {:?}", session_id);
            let result = run_aggregator(
                aggregator_io,
                io,
                client,
                &session_boundaries,
                metrics,
                exit,
            )
            .await;
            let result = match result {
                Ok(_) => Ok(()),
                Err(err) => {
                    error!(target: LOG_TARGET, "Aggregator exited with error: {err}");
                    Err(())
                }
            };
            debug!(target: LOG_TARGET, "Aggregator task stopped for {:?}", session_id);
            result
        }
    };

    let handle =
        spawn_handle.spawn_essential_with_result("aleph/consensus_session_aggregator", task);
    Task::new(handle, stop)
}
