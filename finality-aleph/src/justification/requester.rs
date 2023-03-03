use std::{fmt, marker::PhantomData, time::Instant};

use aleph_primitives::ALEPH_ENGINE_ID;
use log::{debug, error, info, warn};
use sc_client_api::blockchain::Info;
use sp_api::{BlockId, BlockT, NumberFor};
use sp_runtime::traits::{Header, One, Saturating};

use crate::{
    finalization::BlockFinalizer,
    justification::{
        scheduler::SchedulerActions, versioned_encode, JustificationNotification,
        JustificationRequestScheduler, Verifier,
    },
    metrics::Checkpoint,
    network, BlockHashNum, BlockchainBackend, Metrics,
};

/// Threshold for how many tries are needed so that JustificationRequestStatus is logged
const REPORT_THRESHOLD: u32 = 2;

/// This structure is created for keeping and reporting status of BlockRequester
pub struct JustificationRequestStatus<B: BlockT> {
    block_hash_number: Option<BlockHashNum<B>>,
    block_tries: u32,
    parent: Option<B::Hash>,
    n_children: usize,
    children_tries: u32,
    report_threshold: u32,
}

impl<B: BlockT> JustificationRequestStatus<B> {
    fn new() -> Self {
        Self {
            block_hash_number: None,
            block_tries: 0,
            parent: None,
            n_children: 0,
            children_tries: 0,
            report_threshold: REPORT_THRESHOLD,
        }
    }

    fn save_children(&mut self, hash: B::Hash, n_children: usize) {
        if self.parent == Some(hash) {
            self.children_tries += 1;
        } else {
            self.parent = Some(hash);
            self.children_tries = 1;
        }
        self.n_children = n_children;
    }

    fn save_block(&mut self, hn: BlockHashNum<B>) {
        if self.block_hash_number == Some(hn.clone()) {
            self.block_tries += 1;
        } else {
            self.block_hash_number = Some(hn);
            self.block_tries = 1;
        }
    }

    fn reset(&mut self) {
        self.block_hash_number = None;
        self.block_tries = 0;
        self.parent = None;
        self.n_children = 0;
        self.children_tries = 0;
    }

    fn should_report(&self) -> bool {
        self.block_tries >= self.report_threshold || self.children_tries >= self.report_threshold
    }
}

impl<B: BlockT> fmt::Display for JustificationRequestStatus<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.block_tries >= self.report_threshold {
            if let Some(hn) = &self.block_hash_number {
                write!(
                    f,
                    "tries - {}; requested block number - {}; hash - {};",
                    self.block_tries, hn.num, hn.hash,
                )?;
            }
        }
        if self.children_tries >= self.report_threshold {
            if let Some(parent) = self.parent {
                write!(
                    f,
                    "tries - {}; requested {} children of finalized block {}; ",
                    self.children_tries, self.n_children, parent
                )?;
            }
        }
        Ok(())
    }
}

pub struct BlockRequester<B, RB, S, F, V, BB>
where
    B: BlockT,
    RB: network::RequestBlocks<B> + 'static,
    S: JustificationRequestScheduler,
    F: BlockFinalizer<B>,
    V: Verifier<B>,
    BB: BlockchainBackend<B> + 'static,
{
    block_requester: RB,
    blockchain_backend: BB,
    finalizer: F,
    justification_request_scheduler: S,
    metrics: Option<Metrics<<B::Header as Header>::Hash>>,
    request_status: JustificationRequestStatus<B>,
    _phantom: PhantomData<V>,
}

impl<B, RB, S, F, V, BB> BlockRequester<B, RB, S, F, V, BB>
where
    B: BlockT,
    RB: network::RequestBlocks<B> + 'static,
    S: JustificationRequestScheduler,
    F: BlockFinalizer<B>,
    V: Verifier<B>,
    BB: BlockchainBackend<B> + 'static,
{
    pub fn new(
        block_requester: RB,
        blockchain_backend: BB,
        finalizer: F,
        justification_request_scheduler: S,
        metrics: Option<Metrics<<B::Header as Header>::Hash>>,
    ) -> Self {
        BlockRequester {
            block_requester,
            blockchain_backend,
            finalizer,
            justification_request_scheduler,
            metrics,
            request_status: JustificationRequestStatus::new(),
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
                self.request_status.reset();
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

    pub fn status_report(&self) {
        if self.request_status.should_report() {
            info!(target: "aleph-justification", "Justification requester status report: {}", self.request_status);
        }
    }

    pub fn request_justification(&mut self, wanted: NumberFor<B>) {
        match self.justification_request_scheduler.schedule_action() {
            SchedulerActions::Request => {
                let info = self.blockchain_backend.info();
                self.request_children(&info);
                self.request_wanted(wanted, &info);
            }
            SchedulerActions::ClearQueue => {
                debug!(target: "aleph-justification", "Clearing justification request queue");
                self.block_requester.clear_justification_requests();
            }
            SchedulerActions::Wait => (),
        }
    }

    pub fn finalized_number(&self) -> NumberFor<B> {
        self.blockchain_backend.info().finalized_number
    }

    fn do_request(&mut self, hash: &<B as BlockT>::Hash, num: NumberFor<B>) {
        debug!(target: "aleph-justification",
               "We have block {:?} with hash {:?}. Requesting justification.", num, hash);
        self.justification_request_scheduler.on_request_sent();
        self.block_requester.request_justification(hash, num);
    }

    // We request justifications for all the children of last finalized block.
    // Assuming that we request at the same pace that finalization is progressing, it ensures
    // that we are up to date with finalization.
    // We also request the child that it's on the same branch as top_wanted since a fork may happen
    // somewhere in between them.
    fn request_children(&mut self, info: &Info<B>) {
        let finalized_hash = info.finalized_hash;
        let finalized_number = info.finalized_number;

        let children = self.blockchain_backend.children(finalized_hash);

        if !children.is_empty() {
            self.request_status
                .save_children(finalized_hash, children.len());
        }

        for child in &children {
            self.do_request(child, finalized_number + NumberFor::<B>::one());
        }
    }

    // This request is important in the case when we are far behind and want to catch up.
    fn request_wanted(&mut self, mut top_wanted: NumberFor<B>, info: &Info<B>) {
        let best_number = info.best_number;
        if best_number <= top_wanted {
            // most probably block best_number is not yet finalized
            top_wanted = best_number.saturating_sub(NumberFor::<B>::one());
        }
        let finalized_number = info.finalized_number;
        // We know that top_wanted >= finalized_number, so
        // - if top_wanted == finalized_number, then we don't want to request it
        // - if top_wanted == finalized_number + 1, then we already requested it
        if top_wanted <= finalized_number + NumberFor::<B>::one() {
            return;
        }
        match self.blockchain_backend.header(BlockId::Number(top_wanted)) {
            Ok(Some(header)) => {
                let hash = header.hash();
                let num = *header.number();
                self.do_request(&hash, num);
                self.request_status.save_block((hash, num).into());
            }
            Ok(None) => {
                warn!(target: "aleph-justification", "Cancelling request, because we don't have block {:?}.", top_wanted);
            }
            Err(err) => {
                warn!(target: "aleph-justification", "Cancelling request, because fetching block {:?} failed {:?}.", top_wanted, err);
            }
        }
    }
}
