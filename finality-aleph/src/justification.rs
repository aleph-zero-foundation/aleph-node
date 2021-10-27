use crate::{
    crypto::{AuthorityVerifier, Signature},
    finalization::finalize_block,
    last_block_of_session,
    metrics::Checkpoint,
    network, session_id_from_block_num, Metrics, SessionId, SessionMap,
};
use aleph_bft::SignatureSet;
use aleph_primitives::{SessionPeriod, ALEPH_ENGINE_ID};
use codec::{Decode, Encode};
use futures::{channel::mpsc, StreamExt};
use futures_timer::Delay;
use log::{debug, error, warn};
use parking_lot::Mutex;
use sc_client_api::backend::Backend;
use sp_api::{BlockId, BlockT, NumberFor};
use sp_runtime::traits::Header;
use std::{
    marker::PhantomData,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::time::timeout;

/// A proof of block finality, currently in the form of a sufficiently long list of signatures.
#[derive(Clone, Encode, Decode, Debug)]
pub struct AlephJustification {
    pub(crate) signature: SignatureSet<Signature>,
}

impl AlephJustification {
    pub(crate) fn new<Block: BlockT>(signature: SignatureSet<Signature>) -> Self {
        Self { signature }
    }

    pub(crate) fn verify<Block: BlockT>(
        aleph_justification: &AlephJustification,
        block_hash: Block::Hash,
        multi_verifier: &AuthorityVerifier,
    ) -> bool {
        if !multi_verifier.is_complete(&block_hash.encode()[..], &aleph_justification.signature) {
            debug!(target: "afa", "Bad justification for block hash #{:?} {:?}", block_hash, aleph_justification);
            return false;
        }
        true
    }
}

pub(crate) struct ChainCadence {
    pub session_period: SessionPeriod,
    pub justifications_cadence: Duration,
}

/// A notification for sending justifications over the network.
pub struct JustificationNotification<Block>
where
    Block: BlockT,
{
    /// The justification itself.
    pub justification: AlephJustification,
    /// The hash of the finalized block.
    pub hash: Block::Hash,
    /// The ID of the finalized block.
    pub number: NumberFor<Block>,
}

pub(crate) struct JustificationHandler<B, RB, C, BE>
where
    B: BlockT,
    RB: network::RequestBlocks<B> + 'static,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    BE: Backend<B> + 'static,
{
    session_authorities: Arc<Mutex<SessionMap>>,
    chain_cadence: ChainCadence,
    block_requester: RB,
    client: Arc<C>,
    last_request_time: Instant,
    last_finalization_time: Instant,
    metrics: Option<Metrics<B::Header>>,
    phantom: PhantomData<BE>,
}

impl<B, RB, C, BE> JustificationHandler<B, RB, C, BE>
where
    B: BlockT,
    RB: network::RequestBlocks<B> + 'static,
    C: crate::ClientForAleph<B, BE> + Send + Sync + 'static,
    BE: Backend<B> + 'static,
{
    pub(crate) fn new(
        session_authorities: Arc<Mutex<SessionMap>>,
        chain_cadence: ChainCadence,
        block_requester: RB,
        client: Arc<C>,
        metrics: Option<Metrics<B::Header>>,
    ) -> Self {
        Self {
            session_authorities,
            chain_cadence,
            block_requester,
            client,
            last_request_time: Instant::now(),
            last_finalization_time: Instant::now(),
            metrics,
            phantom: PhantomData,
        }
    }

    pub(crate) fn handle_justification_notification(
        &mut self,
        notification: JustificationNotification<B>,
        verifier: AuthorityVerifier,
        last_finalized: NumberFor<B>,
        stop_h: NumberFor<B>,
    ) {
        let num = notification.number;
        let block_hash = notification.hash;
        if AlephJustification::verify::<B>(
            &notification.justification,
            notification.hash,
            &verifier,
        ) {
            if num > last_finalized && num <= stop_h {
                debug!(target: "afa", "Finalizing block {:?} {:?}", num, block_hash);
                let finalization_res = finalize_block(
                    self.client.clone(),
                    block_hash,
                    num,
                    Some((ALEPH_ENGINE_ID, notification.justification.encode())),
                );
                match finalization_res {
                    Ok(()) => {
                        self.last_finalization_time = Instant::now();
                        debug!(target: "afa", "Successfully finalized {:?}", num);
                        if let Some(metrics) = &self.metrics {
                            metrics.report_block(
                                block_hash,
                                self.last_finalization_time,
                                Checkpoint::Finalized,
                            );
                        }
                    }
                    Err(e) => {
                        warn!(target: "afa", "Fail in finalization of {:?} {:?} -- {:?}", num, block_hash, e);
                    }
                }
            } else {
                debug!(target: "afa", "Not finalizing block {:?}. Last finalized {:?}, stop_h {:?}", num, last_finalized, stop_h);
            }
        } else {
            error!(target: "afa", "Error when verifying justification for block {:?} {:?}", num, block_hash);
        }
    }

    fn request_justification(&mut self, num: NumberFor<B>) {
        let current_time = Instant::now();

        let ChainCadence {
            justifications_cadence,
            ..
        } = self.chain_cadence;

        if current_time - self.last_finalization_time > justifications_cadence
            && current_time - self.last_request_time > 2 * justifications_cadence
        {
            debug!(target: "afa", "Trying to request block {:?}", num);

            if let Ok(Some(header)) = self.client.header(BlockId::Number(num)) {
                debug!(target: "afa", "We have block {:?} with hash {:?}. Requesting justification.", num, header.hash());
                self.last_request_time = current_time;
                self.block_requester
                    .request_justification(&header.hash(), *header.number());
            } else {
                debug!(target: "afa", "Cancelling request, because we don't have block {:?}.", num);
            }
        }
    }

    pub(crate) async fn run(
        mut self,
        authority_justification_rx: mpsc::UnboundedReceiver<JustificationNotification<B>>,
        import_justification_rx: mpsc::UnboundedReceiver<JustificationNotification<B>>,
    ) {
        let import_stream = import_justification_rx
            .inspect(|_| {
                debug!(target: "afa", "Got justification (import)");
            })
            .chain(futures::stream::iter(std::iter::from_fn(|| {
                error!(target: "afa", "Justification (import) stream ended.");
                None
            })));

        let authority_stream = authority_justification_rx
            .inspect(|_| {
                debug!(target: "afa", "Got justification (aggregator)");
            })
            .chain(futures::stream::iter(std::iter::from_fn(|| {
                error!(target: "afa", "Justification (aggregator) stream ended.");
                None
            })));

        let mut notification_stream = futures::stream::select(import_stream, authority_stream);

        let ChainCadence { session_period, .. } = self.chain_cadence;

        loop {
            let last_finalized_number = self.client.info().finalized_number;
            let current_session =
                session_id_from_block_num::<B>(last_finalized_number + 1u32.into(), session_period);
            let stop_h: NumberFor<B> = last_block_of_session::<B>(current_session, session_period);
            let verifier = self.session_verifier(current_session);
            if verifier.is_none() {
                debug!(target: "afa", "Verifier for session {:?} not yet available. Waiting 500ms and will try again ...", current_session);
                Delay::new(Duration::from_millis(500)).await;
                continue;
            }
            let verifier = verifier.expect("We loop until this is some.");

            match timeout(Duration::from_millis(1000), notification_stream.next()).await {
                Ok(Some(notification)) => {
                    self.handle_justification_notification(
                        notification,
                        verifier,
                        last_finalized_number,
                        stop_h,
                    );
                }
                Ok(None) => {
                    error!(target: "afa", "Justification stream ended.");
                    return;
                }
                Err(_) => {
                    //Timeout passed
                }
            }

            self.request_justification(stop_h);
        }
    }

    fn session_verifier(&self, session_id: SessionId) -> Option<AuthorityVerifier> {
        Some(AuthorityVerifier::new(
            self.session_authorities.lock().get(&session_id)?.to_vec(),
        ))
    }
}
