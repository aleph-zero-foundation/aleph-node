use std::{collections::HashSet, iter, time::Duration};

use futures::{channel::mpsc, StreamExt};
use log::{debug, error, trace, warn};
use tokio::time::{interval_at, Instant};

use crate::{
    metrics::Key,
    network::GossipNetwork,
    sync::{
        data::{
            BranchKnowledge, NetworkData, Request, State, VersionWrapper, VersionedNetworkData,
        },
        forest::Interest,
        handler::{Error as HandlerError, Handler, SyncAction},
        task_queue::TaskQueue,
        ticker::Ticker,
        BlockIdFor, BlockIdentifier, ChainStatus, ChainStatusNotification, ChainStatusNotifier,
        Finalizer, Header, Justification, JustificationSubmissions, Verifier, LOG_TARGET,
    },
    Metrics, SessionPeriod,
};

const BROADCAST_COOLDOWN: Duration = Duration::from_millis(600);
const BROADCAST_PERIOD: Duration = Duration::from_secs(5);
const FINALIZATION_STALL_CHECK_PERIOD: Duration = Duration::from_secs(30);

/// A service synchronizing the knowledge about the chain between the nodes.
pub struct Service<
    J: Justification,
    N: GossipNetwork<VersionedNetworkData<J>>,
    CE: ChainStatusNotifier<J::Header>,
    CS: ChainStatus<J>,
    V: Verifier<J>,
    F: Finalizer<J>,
    H: Key,
> {
    network: VersionWrapper<J, N>,
    handler: Handler<N::PeerId, J, CS, V, F>,
    tasks: TaskQueue<BlockIdFor<J>>,
    broadcast_ticker: Ticker,
    chain_events: CE,
    justifications_from_user: mpsc::UnboundedReceiver<J::Unverified>,
    additional_justifications_from_user: mpsc::UnboundedReceiver<J::Unverified>,
    metrics: Metrics<H>,
}

impl<J: Justification> JustificationSubmissions<J> for mpsc::UnboundedSender<J::Unverified> {
    type Error = mpsc::TrySendError<J::Unverified>;

    fn submit(&mut self, justification: J::Unverified) -> Result<(), Self::Error> {
        self.unbounded_send(justification)
    }
}

impl<
        J: Justification,
        N: GossipNetwork<VersionedNetworkData<J>>,
        CE: ChainStatusNotifier<J::Header>,
        CS: ChainStatus<J>,
        V: Verifier<J>,
        F: Finalizer<J>,
        H: Key,
    > Service<J, N, CE, CS, V, F, H>
{
    /// Create a new service using the provided network for communication. Also returns an
    /// interface for submitting additional justifications.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        network: N,
        chain_events: CE,
        chain_status: CS,
        verifier: V,
        finalizer: F,
        period: SessionPeriod,
        additional_justifications_from_user: mpsc::UnboundedReceiver<J::Unverified>,
        metrics: Metrics<H>,
    ) -> Result<(Self, impl JustificationSubmissions<J> + Clone), HandlerError<J, CS, V, F>> {
        let network = VersionWrapper::new(network);
        let handler = Handler::new(chain_status, verifier, finalizer, period)?;
        let tasks = TaskQueue::new();
        let broadcast_ticker = Ticker::new(BROADCAST_PERIOD, BROADCAST_COOLDOWN);
        let (justifications_for_sync, justifications_from_user) = mpsc::unbounded();
        Ok((
            Service {
                network,
                handler,
                tasks,
                broadcast_ticker,
                chain_events,
                justifications_from_user,
                additional_justifications_from_user,
                metrics,
            },
            justifications_for_sync,
        ))
    }

    fn backup_request(&mut self, block_id: BlockIdFor<J>) {
        self.tasks.schedule_in(block_id, Duration::from_secs(5));
    }

    fn delayed_request(&mut self, block_id: BlockIdFor<J>) {
        self.tasks.schedule_in(block_id, Duration::from_millis(500));
    }

    fn request(&mut self, block_id: BlockIdFor<J>) {
        self.tasks.schedule_in(block_id, Duration::ZERO);
    }

    fn broadcast(&mut self) {
        let state = match self.handler.state() {
            Ok(state) => state,
            Err(e) => {
                warn!(
                    target: LOG_TARGET,
                    "Failed to construct own knowledge state: {}.", e
                );
                return;
            }
        };
        trace!(target: LOG_TARGET, "Broadcasting state: {:?}", state);
        self.metrics.report_sync_broadcast();
        let data = NetworkData::StateBroadcast(state);
        if let Err(e) = self.network.broadcast(data) {
            warn!(target: LOG_TARGET, "Error sending broadcast: {}.", e);
        }
    }

    fn send_request_for(
        &mut self,
        block_id: BlockIdFor<J>,
        branch_knowledge: BranchKnowledge<J>,
        peers: HashSet<N::PeerId>,
    ) {
        let state = match self.handler.state() {
            Ok(state) => state,
            Err(e) => {
                warn!(
                    target: LOG_TARGET,
                    "Failed to construct own knowledge state: {}.", e
                );
                return;
            }
        };
        let request = Request::new(block_id, branch_knowledge, state);
        trace!(target: LOG_TARGET, "Sending a request: {:?}", request);
        self.metrics.report_sync_send_request_for();
        let data = NetworkData::Request(request);
        if let Err(e) = self.network.send_to_random(data, peers) {
            warn!(target: LOG_TARGET, "Error sending request: {}.", e);
        }
    }

    fn send_to(&mut self, data: NetworkData<J>, peer: N::PeerId) {
        trace!(
            target: LOG_TARGET,
            "Sending data {:?} to peer {:?}",
            data,
            peer
        );
        self.metrics.report_sync_send_to();
        if let Err(e) = self.network.send_to(data, peer) {
            warn!(target: LOG_TARGET, "Error sending response: {}.", e);
        }
    }

    fn perform_sync_action(&mut self, action: SyncAction<J>, peer: N::PeerId) {
        use SyncAction::*;
        match action {
            Response(data) => self.send_to(data, peer),
            Task(block_id) => self.request(block_id),
            Noop => (),
        }
    }

    fn handle_state(&mut self, state: State<J>, peer: N::PeerId) {
        trace!(
            target: LOG_TARGET,
            "Handling state {:?} received from {:?}.",
            state,
            peer
        );
        self.metrics.report_sync_handle_state();
        match self.handler.handle_state(state, peer.clone()) {
            Ok(action) => self.perform_sync_action(action, peer),
            Err(e) => warn!(
                target: LOG_TARGET,
                "Error handling sync state from {:?}: {}.", peer, e
            ),
        }
    }

    fn handle_justifications(
        &mut self,
        justifications: Vec<J::Unverified>,
        peer: Option<N::PeerId>,
    ) {
        trace!(
            target: LOG_TARGET,
            "Handling {:?} justifications.",
            justifications.len()
        );
        self.metrics.report_sync_handle_justifications();
        let mut previous_block_id = None;
        for justification in justifications {
            let maybe_block_id = match self
                .handler
                .handle_justification(justification, peer.clone())
            {
                Ok(maybe_id) => maybe_id,
                Err(e) => match e {
                    HandlerError::Verifier(e) => {
                        debug!(
                            target: LOG_TARGET,
                            "Could not verify justification from {:?}: {}.",
                            peer.map_or("user".to_string(), |id| format!("{:?}", id)),
                            e
                        );
                        return;
                    }
                    e => {
                        warn!(
                            target: LOG_TARGET,
                            "Error while handling justification from {:?}: {}.",
                            peer.map_or("user".to_string(), |id| format!("{:?}", id)),
                            e
                        );
                        return;
                    }
                },
            };
            if let Some(block_id) = maybe_block_id {
                if let Some(previous_block_id) = previous_block_id {
                    self.backup_request(previous_block_id);
                }
                previous_block_id = Some(block_id);
            }
        }
        if let Some(block_id) = previous_block_id {
            debug!(
                target: LOG_TARGET,
                "Initiating a request for {:?}.", block_id
            );
            self.request(block_id);
        }
    }

    fn handle_request(&mut self, request: Request<J>, peer: N::PeerId) {
        trace!(
            target: LOG_TARGET,
            "Handling a request {:?} from {:?}.",
            request,
            peer
        );
        self.metrics.report_sync_handle_request();
        match self.handler.handle_request(request) {
            Ok(action) => self.perform_sync_action(action, peer),
            Err(e) => {
                warn!(
                    target: LOG_TARGET,
                    "Error handling request from {:?}: {}.", peer, e
                );
            }
        }
    }

    fn handle_network_data(&mut self, data: NetworkData<J>, peer: N::PeerId) {
        use NetworkData::*;
        match data {
            StateBroadcast(state) => self.handle_state(state, peer),
            StateBroadcastResponse(justification, maybe_justification) => {
                self.handle_justifications(
                    iter::once(justification)
                        .chain(maybe_justification)
                        .collect(),
                    Some(peer),
                );
            }
            Request(request) => {
                let state = request.state().clone();
                self.handle_request(request, peer.clone());
                self.handle_state(state, peer)
            }
            RequestResponse(justifications) => {
                self.handle_justifications(justifications, Some(peer))
            }
        }
    }

    fn handle_task(&mut self, block_id: BlockIdFor<J>) {
        trace!(target: LOG_TARGET, "Handling a task for {:?}.", block_id);
        self.metrics.report_sync_handle_task();
        use Interest::*;
        match self.handler.block_state(&block_id) {
            HighestJustified {
                know_most,
                branch_knowledge,
            }
            | TopRequired {
                know_most,
                branch_knowledge,
            } => {
                self.send_request_for(block_id.clone(), branch_knowledge, know_most);
                self.delayed_request(block_id);
            }
            Required {
                know_most,
                branch_knowledge,
            } => {
                self.send_request_for(block_id.clone(), branch_knowledge, know_most);
                self.backup_request(block_id);
            }
            Uninterested => (),
        }
    }

    fn handle_chain_event(&mut self, event: ChainStatusNotification<J::Header>) {
        use ChainStatusNotification::*;
        match event {
            BlockImported(header) => {
                trace!(target: LOG_TARGET, "Handling a new imported block.");
                self.metrics.report_sync_handle_block_imported();
                if let Err(e) = self.handler.block_imported(header) {
                    error!(
                        target: LOG_TARGET,
                        "Error marking block as imported: {}.", e
                    );
                }
            }
            BlockFinalized(_) => {
                trace!(target: LOG_TARGET, "Handling a new finalized block.");
                self.metrics.report_sync_handle_block_finalized();
                if self.broadcast_ticker.try_tick() {
                    self.broadcast();
                }
            }
        }
    }

    /// Stay synchronized.
    pub async fn run(mut self) {
        // TODO(A0-1758): Remove after finishing the sync rewrite.
        let mut stall_ticker = interval_at(
            Instant::now() + FINALIZATION_STALL_CHECK_PERIOD,
            FINALIZATION_STALL_CHECK_PERIOD,
        );
        let mut last_top_number = 0;
        loop {
            tokio::select! {
                maybe_data = self.network.next() => match maybe_data {
                    Ok((data, peer)) => self.handle_network_data(data, peer),
                    Err(e) => warn!(target: LOG_TARGET, "Error receiving data from network: {}.", e),
                },
                Some(block_id) = self.tasks.pop() => self.handle_task(block_id),
                _ = self.broadcast_ticker.wait_and_tick() => self.broadcast(),
                maybe_event = self.chain_events.next() => match maybe_event {
                    Ok(chain_event) => self.handle_chain_event(chain_event),
                    Err(e) => warn!(target: LOG_TARGET, "Error when receiving a chain event: {}.", e),
                },
                maybe_justification = self.justifications_from_user.next() => match maybe_justification {
                    Some(justification) => {
                        debug!(target: LOG_TARGET, "Received new justification from user: {:?}.", justification);
                        self.handle_justifications(vec![justification], None);
                    },
                    None => warn!(target: LOG_TARGET, "Channel with justifications from user closed."),
                },
                maybe_justification = self.additional_justifications_from_user.next() => match maybe_justification {
                    Some(justification) => {
                        debug!(target: LOG_TARGET, "Received new additional justification from user: {:?}.", justification);
                        self.handle_justifications(Vec::from([justification]), None)
                    },
                    None => warn!(target: LOG_TARGET, "Channel with additional justifications from user closed."),
                },
                _ = stall_ticker.tick() => {
                    match self.handler.state() {
                        Ok(state) => {
                            let top_number = state.top_justification().id().number();
                            if top_number == last_top_number {
                                error!(
                                    target: LOG_TARGET,
                                    "Sync stall detected, recreating the Forest."
                                );
                                if let Err(e) = self.handler.refresh_forest() {
                                    error!(
                                        target: LOG_TARGET,
                                        "Error when recreating the Forest: {}.", e
                                    );
                                }
                            } else {
                                last_top_number = top_number;
                            }
                        },
                        Err(e) => error!(
                            target: LOG_TARGET,
                            "Error when retrieving Handler state: {}.", e
                        ),
                    }
                }
            }
        }
    }
}
