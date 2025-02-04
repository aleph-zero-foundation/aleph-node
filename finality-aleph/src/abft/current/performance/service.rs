use std::collections::HashMap;

use current_aleph_bft::NodeCount;
use futures::{
    channel::{mpsc, oneshot},
    StreamExt,
};
use log::{debug, error, warn};
use parity_scale_codec::Encode;
use sp_runtime::traits::Hash as _;

use crate::{
    abft::{
        current::performance::{scorer::Scorer, Batch},
        LOG_TARGET,
    },
    aleph_primitives::{
        crypto::SignatureSet, AuthoritySignature, Hash, Hashing, RawScore, Score, ScoreNonce,
    },
    data_io::AlephData,
    metrics::ScoreMetrics,
    party::manager::Runnable,
    runtime_api::RuntimeApi,
    Hasher, SessionId, UnverifiedHeader,
};

struct FinalizationWrapper<UH, FH>
where
    UH: UnverifiedHeader,
    FH: current_aleph_bft::FinalizationHandler<AlephData<UH>>,
{
    finalization_handler: FH,
    batches_for_scorer: mpsc::UnboundedSender<Batch<UH>>,
}

impl<UH, FH> FinalizationWrapper<UH, FH>
where
    UH: UnverifiedHeader,
    FH: current_aleph_bft::FinalizationHandler<AlephData<UH>>,
{
    fn new(finalization_handler: FH, batches_for_scorer: mpsc::UnboundedSender<Batch<UH>>) -> Self {
        FinalizationWrapper {
            finalization_handler,
            batches_for_scorer,
        }
    }
}

impl<UH, FH> current_aleph_bft::UnitFinalizationHandler for FinalizationWrapper<UH, FH>
where
    UH: UnverifiedHeader,
    FH: current_aleph_bft::FinalizationHandler<AlephData<UH>>,
{
    type Data = AlephData<UH>;
    type Hasher = Hasher;

    fn batch_finalized(&mut self, batch: Batch<UH>) {
        for unit in &batch {
            if let Some(data) = &unit.data {
                self.finalization_handler.data_finalized(data.clone())
            }
        }
        if let Err(err) = self.batches_for_scorer.unbounded_send(batch) {
            warn!(target: LOG_TARGET, "Failed to send ABFT batch to performance scoring: {}.", err);
        }
    }
}

/// A service computing the performance score of ABFT nodes based on batches of ordered units.
pub struct Service<UH, RA>
where
    UH: UnverifiedHeader,
    RA: RuntimeApi,
{
    my_index: usize,
    session_id: SessionId,
    score_submission_period: u32,
    batches_from_abft: mpsc::UnboundedReceiver<Batch<UH>>,
    hashes_for_aggregator: mpsc::UnboundedSender<Hash>,
    signatures_from_aggregator: mpsc::UnboundedReceiver<(Hash, SignatureSet<AuthoritySignature>)>,
    runtime_api: RA,
    pending_scores: HashMap<Hash, Score>,
    nonce: ScoreNonce,
    scorer: Scorer,
    metrics: ScoreMetrics,
}

pub struct ServiceIO {
    pub hashes_for_aggregator: mpsc::UnboundedSender<Hash>,
    pub signatures_from_aggregator:
        mpsc::UnboundedReceiver<(Hash, SignatureSet<AuthoritySignature>)>,
}

impl<UH, RA> Service<UH, RA>
where
    UH: UnverifiedHeader,
    RA: RuntimeApi,
{
    /// Create a new service, together with a unit finalization handler that should be passed to
    /// ABFT. It will wrap the provided finalization handler and call it in the background.
    #[allow(clippy::too_many_arguments)]
    pub fn new<FH>(
        my_index: usize,
        n_members: usize,
        session_id: SessionId,
        score_submission_period: u32,
        finalization_handler: FH,
        io: ServiceIO,
        runtime_api: RA,
        metrics: ScoreMetrics,
    ) -> (
        Self,
        impl current_aleph_bft::UnitFinalizationHandler<Data = AlephData<UH>, Hasher = Hasher>,
    )
    where
        FH: current_aleph_bft::FinalizationHandler<AlephData<UH>>,
    {
        let ServiceIO {
            hashes_for_aggregator,
            signatures_from_aggregator,
        } = io;
        let (batches_for_us, batches_from_abft) = mpsc::unbounded();
        (
            Service {
                my_index,
                session_id,
                score_submission_period,
                batches_from_abft,
                hashes_for_aggregator,
                signatures_from_aggregator,
                runtime_api,
                pending_scores: HashMap::new(),
                nonce: 1,
                scorer: Scorer::new(NodeCount(n_members)),
                metrics,
            },
            FinalizationWrapper::new(finalization_handler, batches_for_us),
        )
    }

    fn make_score(&mut self, points: RawScore) -> Score {
        let result = Score {
            session_id: self.session_id.0,
            nonce: self.nonce,
            points,
        };
        self.nonce += 1;
        result
    }
}

#[async_trait::async_trait]
impl<UH, RA> Runnable for Service<UH, RA>
where
    UH: UnverifiedHeader,
    RA: RuntimeApi,
{
    async fn run(mut self, mut exit: oneshot::Receiver<()>) {
        let mut batch_counter = 1;
        loop {
            tokio::select! {
                maybe_batch = self.batches_from_abft.next() => {
                    let points = match maybe_batch {
                        Some(batch) => self.scorer.process_batch(batch),
                        None => {
                            error!(target: LOG_TARGET, "Batches' channel closed, ABFT performance scoring terminating.");
                            break;
                        },
                    };
                    self.metrics.report_score(points[self.my_index]);
                    if batch_counter % self.score_submission_period == 0 {
                        let score = self.make_score(points);
                        let score_hash = Hashing::hash_of(&score.encode());
                        debug!(target: LOG_TARGET, "Gathering signature under ABFT score: {:?}.", score);
                        self.pending_scores.insert(score_hash, score);
                        if let Err(e) = self.hashes_for_aggregator.unbounded_send(score_hash) {
                            error!(target: LOG_TARGET, "Failed to send score hash to signature aggregation: {}.", e);
                            break;
                        }
                    }
                    batch_counter += 1;
                }
                maybe_signed = self.signatures_from_aggregator.next() => {
                    match maybe_signed {
                        Some((hash, signature)) => {
                            match self.pending_scores.remove(&hash) {
                                Some(score) => {
                                    if let Err(e) = self.runtime_api.submit_abft_score(score, signature) {
                                        warn!(target: LOG_TARGET, "Failed to submit performance score to chain: {}.", e);
                                    }
                                },
                                None => {
                                    warn!(target: LOG_TARGET, "Received multisigned hash for unknown performance score, this shouldn't ever happen.");
                                },
                            }
                        },
                        None => {
                            error!(target: LOG_TARGET, "Signatures' channel closed, ABFT performance scoring terminating.");
                            break;
                        },
                    }
                }
                _ = &mut exit => {
                    debug!(target: LOG_TARGET, "ABFT performance scoring task received exit signal. Terminating.");
                    break;
                }
            }
        }
    }
}
