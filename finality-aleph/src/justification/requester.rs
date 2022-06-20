use crate::{
    finalization::BlockFinalizer,
    justification::{
        scheduler::SchedulerActions, versioned_encode, JustificationNotification,
        JustificationRequestScheduler, Verifier,
    },
    metrics::Checkpoint,
    network, Metrics,
};
use aleph_primitives::ALEPH_ENGINE_ID;
use log::{debug, error, warn};
use sc_client_api::HeaderBackend;
use sp_api::{BlockId, BlockT, NumberFor};
use sp_runtime::traits::Header;
use std::{marker::PhantomData, sync::Arc, time::Instant};

pub struct BlockRequester<B, RB, C, S, F, V>
where
    B: BlockT,
    RB: network::RequestBlocks<B> + 'static,
    C: HeaderBackend<B> + Send + Sync + 'static,
    S: JustificationRequestScheduler,
    F: BlockFinalizer<B>,
    V: Verifier<B>,
{
    block_requester: RB,
    client: Arc<C>,
    finalizer: F,
    justification_request_scheduler: S,
    metrics: Option<Metrics<<B::Header as Header>::Hash>>,
    min_allowed_delay: NumberFor<B>,
    _phantom: PhantomData<V>,
}

impl<B, RB, C, S, F, V> BlockRequester<B, RB, C, S, F, V>
where
    B: BlockT,
    RB: network::RequestBlocks<B> + 'static,
    C: HeaderBackend<B> + Send + Sync + 'static,
    S: JustificationRequestScheduler,
    F: BlockFinalizer<B>,
    V: Verifier<B>,
{
    pub fn new(
        block_requester: RB,
        client: Arc<C>,
        finalizer: F,
        justification_request_scheduler: S,
        metrics: Option<Metrics<<B::Header as Header>::Hash>>,
        min_allowed_delay: NumberFor<B>,
    ) -> Self {
        BlockRequester {
            block_requester,
            client,
            finalizer,
            justification_request_scheduler,
            metrics,
            min_allowed_delay,
            _phantom: PhantomData,
        }
    }

    pub fn handle_justification_notification(
        &mut self,
        notification: JustificationNotification<B>,
        verifier: V,
        last_finalized: NumberFor<B>,
        stop_h: NumberFor<B>,
    ) {
        let JustificationNotification {
            justification,
            number,
            hash,
        } = notification;

        if number <= last_finalized || number > stop_h {
            debug!(target: "aleph-justification", "Not finalizing block {:?}. Last finalized {:?}, stop_h {:?}", number, last_finalized, stop_h);
            return;
        };

        if !(verifier.verify(&justification, hash)) {
            warn!(target: "aleph-justification", "Error when verifying justification for block {:?} {:?}", number, hash);
            return;
        };

        debug!(target: "aleph-justification", "Finalizing block {:?} {:?}", number, hash);
        let finalization_res = self.finalizer.finalize_block(
            hash,
            number,
            Some((ALEPH_ENGINE_ID, versioned_encode(justification))),
        );
        match finalization_res {
            Ok(()) => {
                self.justification_request_scheduler.on_block_finalized();
                debug!(target: "aleph-justification", "Successfully finalized {:?}", number);
                if let Some(metrics) = &self.metrics {
                    metrics.report_block(hash, Instant::now(), Checkpoint::Finalized);
                }
            }
            Err(e) => {
                error!(target: "aleph-justification", "Fail in finalization of {:?} {:?} -- {:?}", number, hash, e);
            }
        }
    }

    pub fn request_justification(&mut self, num: NumberFor<B>) {
        match self.justification_request_scheduler.schedule_action() {
            SchedulerActions::Request => {
                let num = if num > self.client.info().best_number
                    && self.client.info().best_number > self.min_allowed_delay
                {
                    self.client.info().best_number - self.min_allowed_delay
                } else {
                    num
                };

                debug!(target: "aleph-justification", "Trying to request block {:?}", num);

                if let Ok(Some(header)) = self.client.header(BlockId::Number(num)) {
                    debug!(target: "aleph-justification", "We have block {:?} with hash {:?}. Requesting justification.", num, header.hash());
                    self.justification_request_scheduler.on_request_sent();
                    self.block_requester
                        .request_justification(&header.hash(), *header.number());
                } else {
                    debug!(target: "aleph-justification", "Cancelling request, because we don't have block {:?}.", num);
                }
            }
            SchedulerActions::ClearQueue => {
                debug!(target: "aleph-justification", "Clearing queue");
                self.block_requester.clear_justification_requests();
            }
            SchedulerActions::Wait => (),
        }
    }

    pub fn finalized_number(&self) -> NumberFor<B> {
        self.client.info().finalized_number
    }
}
