use std::{collections::HashSet, time::Duration};

use futures::{channel::mpsc, StreamExt};
use log::{debug, error, trace, warn};
use substrate_prometheus_endpoint::Registry;

use crate::{
    block::{
        Block, BlockImport, ChainStatus, ChainStatusNotification, ChainStatusNotifier,
        EquivocationProof, Finalizer, Header, HeaderVerifier, Justification, JustificationVerifier,
        UnverifiedHeader, UnverifiedHeaderFor,
    },
    network::GossipNetwork,
    session::SessionBoundaryInfo,
    sync::{
        data::{
            NetworkData, PreRequest, Request, ResponseItem, ResponseItems, State, VersionWrapper,
            VersionedNetworkData,
        },
        forest::ExtensionRequest,
        handler::{Action, DatabaseIO, Error as HandlerError, HandleStateAction, Handler},
        message_limiter::{Error as MsgLimiterError, MsgLimiter},
        metrics::{Event, Metrics},
        task_queue::TaskQueue,
        tasks::{Action as TaskAction, RequestTask},
        ticker::Ticker,
        BlockId, JustificationSubmissions, LegacyRequestBlocks, RequestBlocks, LOG_TARGET,
    },
    SyncOracle,
};

const BROADCAST_COOLDOWN: Duration = Duration::from_millis(600);
const CHAIN_EXTENSION_COOLDOWN: Duration = Duration::from_millis(300);
const TICK_PERIOD: Duration = Duration::from_secs(5);

pub struct IO<B, J, N, CE, CS, F, BI>
where
    J: Justification,
    B: Block<UnverifiedHeader = UnverifiedHeaderFor<J>>,
    N: GossipNetwork<VersionedNetworkData<B, J>>,
    CE: ChainStatusNotifier<J::Header>,
    CS: ChainStatus<B, J>,
    F: Finalizer<J>,
    BI: BlockImport<B>,
{
    network: N,
    chain_events: CE,
    sync_oracle: SyncOracle,
    additional_justifications_from_user: mpsc::UnboundedReceiver<J::Unverified>,
    blocks_from_creator: mpsc::UnboundedReceiver<B>,
    database_io: DatabaseIO<B, J, CS, F, BI>,
}

impl<B, J, N, CE, CS, F, BI> IO<B, J, N, CE, CS, F, BI>
where
    J: Justification,
    B: Block<UnverifiedHeader = UnverifiedHeaderFor<J>>,
    N: GossipNetwork<VersionedNetworkData<B, J>>,
    CE: ChainStatusNotifier<J::Header>,
    CS: ChainStatus<B, J>,
    F: Finalizer<J>,
    BI: BlockImport<B>,
{
    pub fn new(
        database_io: DatabaseIO<B, J, CS, F, BI>,
        network: N,
        chain_events: CE,
        sync_oracle: SyncOracle,
        additional_justifications_from_user: mpsc::UnboundedReceiver<J::Unverified>,
        blocks_from_creator: mpsc::UnboundedReceiver<B>,
    ) -> Self {
        IO {
            network,
            chain_events,
            sync_oracle,
            additional_justifications_from_user,
            blocks_from_creator,
            database_io,
        }
    }
}

/// A service synchronizing the knowledge about the chain between the nodes.
pub struct Service<B, J, N, CE, CS, V, F, BI>
where
    J: Justification,
    B: Block<UnverifiedHeader = UnverifiedHeaderFor<J>>,
    N: GossipNetwork<VersionedNetworkData<B, J>>,
    CE: ChainStatusNotifier<J::Header>,
    CS: ChainStatus<B, J>,
    V: JustificationVerifier<J> + HeaderVerifier<J::Header>,
    F: Finalizer<J>,
    BI: BlockImport<B>,
{
    network: VersionWrapper<B, J, N>,
    handler: Handler<B, N::PeerId, J, CS, V, F, BI>,
    tasks: TaskQueue<RequestTask>,
    broadcast_ticker: Ticker,
    chain_extension_ticker: Ticker,
    chain_events: CE,
    justifications_from_user: mpsc::UnboundedReceiver<J::Unverified>,
    additional_justifications_from_user: mpsc::UnboundedReceiver<J::Unverified>,
    block_requests_from_user: mpsc::UnboundedReceiver<B::UnverifiedHeader>,
    legacy_block_requests_from_user: mpsc::UnboundedReceiver<BlockId>,
    blocks_from_creator: mpsc::UnboundedReceiver<B>,
    metrics: Metrics,
}

impl<J: Justification> JustificationSubmissions<J> for mpsc::UnboundedSender<J::Unverified> {
    type Error = mpsc::TrySendError<J::Unverified>;

    fn submit(&mut self, justification: J::Unverified) -> Result<(), Self::Error> {
        self.unbounded_send(justification)
    }
}

// TODO(A0-3494): This will be unnecessary, just impl the trait for the sender.
#[derive(Clone)]
struct CompatibilityRequestBlocks<UH: UnverifiedHeader> {
    current: mpsc::UnboundedSender<UH>,
    legacy: mpsc::UnboundedSender<BlockId>,
}

impl<UH: UnverifiedHeader> LegacyRequestBlocks for CompatibilityRequestBlocks<UH> {
    type Error = mpsc::TrySendError<BlockId>;

    fn request_block(&self, block_id: BlockId) -> Result<(), Self::Error> {
        self.legacy.unbounded_send(block_id)
    }
}

impl<UH: UnverifiedHeader> RequestBlocks<UH> for CompatibilityRequestBlocks<UH> {
    type Error = mpsc::TrySendError<UH>;

    fn request_block(&self, header: UH) -> Result<(), Self::Error> {
        self.current.unbounded_send(header)
    }
}

impl<B, J, N, CE, CS, V, F, BI> Service<B, J, N, CE, CS, V, F, BI>
where
    J: Justification,
    B: Block<UnverifiedHeader = UnverifiedHeaderFor<J>>,
    N: GossipNetwork<VersionedNetworkData<B, J>>,
    CE: ChainStatusNotifier<J::Header>,
    CS: ChainStatus<B, J>,
    V: JustificationVerifier<J> + HeaderVerifier<J::Header>,
    F: Finalizer<J>,
    BI: BlockImport<B>,
{
    /// Create a new service using the provided network for communication.
    /// Also returns an interface for submitting additional justifications,
    /// and an interface for requesting blocks.
    pub fn new(
        verifier: V,
        session_info: SessionBoundaryInfo,
        io: IO<B, J, N, CE, CS, F, BI>,
        metrics_registry: Option<Registry>,
    ) -> Result<
        (
            Self,
            impl JustificationSubmissions<J> + Clone,
            impl RequestBlocks<B::UnverifiedHeader> + LegacyRequestBlocks,
        ),
        HandlerError<B, J, CS, V, F>,
    > {
        let IO {
            network,
            chain_events,
            sync_oracle,
            additional_justifications_from_user,
            blocks_from_creator,
            database_io,
        } = io;
        let network = VersionWrapper::new(network);
        let handler = Handler::new(database_io, verifier, sync_oracle, session_info)?;
        let tasks = TaskQueue::new();
        let broadcast_ticker = Ticker::new(TICK_PERIOD, BROADCAST_COOLDOWN);
        let chain_extension_ticker = Ticker::new(TICK_PERIOD, CHAIN_EXTENSION_COOLDOWN);
        let (justifications_for_sync, justifications_from_user) = mpsc::unbounded();
        let (block_requests_for_sync, block_requests_from_user) = mpsc::unbounded();
        let (legacy_block_requests_for_sync, legacy_block_requests_from_user) = mpsc::unbounded();
        let metrics = match Metrics::new(metrics_registry) {
            Ok(metrics) => metrics,
            Err(e) => {
                warn!(target: LOG_TARGET, "Failed to create metrics: {}.", e);
                Metrics::noop()
            }
        };

        Ok((
            Service {
                network,
                handler,
                tasks,
                broadcast_ticker,
                chain_extension_ticker,
                chain_events,
                justifications_from_user,
                additional_justifications_from_user,
                blocks_from_creator,
                block_requests_from_user,
                legacy_block_requests_from_user,
                metrics,
            },
            justifications_for_sync,
            CompatibilityRequestBlocks {
                current: block_requests_for_sync,
                legacy: legacy_block_requests_for_sync,
            },
        ))
    }

    fn request_block(&mut self, block_id: BlockId) {
        debug!(
            target: LOG_TARGET,
            "Initiating a request for block {:?}.", block_id
        );
        self.tasks
            .schedule_in(RequestTask::new(block_id), Duration::ZERO);
    }

    fn broadcast(&mut self) {
        self.metrics.report_event(Event::Broadcast);
        self.broadcast_ticker.reset();
        let state = match self.handler.state() {
            Ok(state) => state,
            Err(e) => {
                self.metrics.report_event_error(Event::Broadcast);
                warn!(
                    target: LOG_TARGET,
                    "Failed to construct own knowledge state: {}.", e
                );
                return;
            }
        };
        trace!(target: LOG_TARGET, "Broadcasting state: {:?}", state);

        let data = NetworkData::StateBroadcast(state);
        if let Err(e) = self.network.broadcast(data) {
            self.metrics.report_event_error(Event::Broadcast);
            warn!(target: LOG_TARGET, "Error sending broadcast: {}.", e)
        }
    }

    fn request_favourite_extension(&mut self, know_most: HashSet<N::PeerId>) {
        self.metrics.report_event(Event::SendExtensionRequest);
        let data = match self.handler.state() {
            Ok(state) => NetworkData::ChainExtensionRequest(state),
            Err(e) => {
                self.metrics.report_event_error(Event::SendExtensionRequest);
                warn!(
                    target: LOG_TARGET,
                    "Error producing state for chain extension request: {}.", e
                );
                return;
            }
        };
        match self.network.send_to_random(data, know_most) {
            Ok(()) => self.chain_extension_ticker.reset(),
            Err(e) => {
                self.metrics.report_event_error(Event::SendExtensionRequest);
                warn!(
                    target: LOG_TARGET,
                    "Error sending chain extension request: {}.", e
                );
            }
        }
    }

    fn request_chain_extension(&mut self, force: bool) {
        use ExtensionRequest::*;
        match self.handler.extension_request() {
            FavouriteBlock { know_most } => self.request_favourite_extension(know_most),
            HighestJustified {
                header,
                know_most,
                branch_knowledge,
            } => {
                self.send_request(PreRequest::new(
                    header.into_unverified(),
                    branch_knowledge,
                    know_most,
                ));
                self.chain_extension_ticker.reset();
            }
            Noop => {
                if force {
                    self.request_favourite_extension(HashSet::new());
                }
            }
        }
    }

    fn try_request_chain_extension(&mut self) {
        if self.chain_extension_ticker.try_tick() {
            self.request_chain_extension(false);
        }
    }

    fn send_request(&mut self, pre_request: PreRequest<UnverifiedHeaderFor<J>, N::PeerId>) {
        self.metrics.report_event(Event::SendRequest);
        let state = match self.handler.state() {
            Ok(state) => state,
            Err(e) => {
                self.metrics.report_event_error(Event::SendRequest);
                warn!(
                    target: LOG_TARGET,
                    "Failed to construct own knowledge state: {}.", e
                );
                return;
            }
        };
        let (request, peers) = pre_request.with_state(state);
        trace!(target: LOG_TARGET, "Sending a request: {:?}", request);
        let data = NetworkData::Request(request);

        if let Err(e) = self.network.send_to_random(data, peers) {
            self.metrics.report_event_error(Event::SendRequest);
            warn!(target: LOG_TARGET, "Error sending request: {}.", e);
        }
    }

    fn send_to(&mut self, data: NetworkData<B, J>, peer: N::PeerId) {
        self.metrics.report_event(Event::SendTo);
        trace!(
            target: LOG_TARGET,
            "Sending data {:?} to peer {:?}",
            data,
            peer
        );
        if let Err(e) = self.network.send_to(data, peer) {
            self.metrics.report_event_error(Event::SendTo);
            warn!(target: LOG_TARGET, "Error sending response: {}.", e);
        }
    }

    fn process_equivocation_proofs<I: IntoIterator<Item = V::EquivocationProof>>(&self, proofs: I) {
        for proof in proofs {
            warn!(target: LOG_TARGET, "Equivocation detected: {proof}");
            if proof.are_we_equivocating() {
                panic!("We are equivocating, which is ILLEGAL - shutting down the node. This is probably caused by running two instances of the node with the same set of credentials. Make sure that you are running ONLY ONE instance of the node. If the problem persists, contact the Aleph Zero developers on Discord.");
            }
        }
    }

    fn handle_state(&mut self, state: State<J>, peer: N::PeerId) {
        self.metrics.report_event(Event::HandleState);
        use HandleStateAction::*;
        trace!(
            target: LOG_TARGET,
            "Handling state {:?} received from {:?}.",
            state,
            peer
        );
        match self.handler.handle_state(state, peer.clone()) {
            Ok((action, maybe_proof)) => {
                self.process_equivocation_proofs(maybe_proof);
                match action {
                    Response(data) => self.send_to(data, peer),
                    ExtendChain => self.try_request_chain_extension(),
                    Noop => (),
                };
            }
            Err(e) => {
                self.metrics.report_event_error(Event::HandleState);
                match e {
                    HandlerError::JustificationVerifier(e) => debug!(
                        target: LOG_TARGET,
                        "Could not verify justification data in sync state from {:?}: {}.", peer, e
                    ),
                    HandlerError::HeaderVerifier(e) => debug!(
                        target: LOG_TARGET,
                        "Could not verify header data in sync state from {:?}: {}.", peer, e
                    ),
                    e => warn!(
                        target: LOG_TARGET,
                        "Failed to handle sync state from {:?}: {}.", peer, e
                    ),
                }
            }
        }
    }

    fn handle_state_response(
        &mut self,
        justification: J::Unverified,
        maybe_justification: Option<J::Unverified>,
        peer: N::PeerId,
    ) {
        trace!(
            target: LOG_TARGET,
            "Handling state response {:?} {:?} received from {:?}.",
            justification,
            maybe_justification,
            peer
        );
        self.metrics.report_event(Event::HandleStateResponse);
        let (new_info, maybe_error) =
            self.handler
                .handle_state_response(justification, maybe_justification, peer.clone());
        match maybe_error {
            Some(HandlerError::JustificationVerifier(e)) => debug!(
                target: LOG_TARGET,
                "Could not verify justification in sync state from {:?}: {}.", peer, e
            ),
            Some(HandlerError::HeaderVerifier(e)) => debug!(
                target: LOG_TARGET,
                "Could not verify header in sync state from {:?}: {}.", peer, e
            ),
            Some(e) => warn!(
                target: LOG_TARGET,
                "Failed to handle sync state response from {:?}: {}.", peer, e
            ),
            _ => {}
        }
        if new_info {
            self.try_request_chain_extension();
        }
    }

    fn handle_justification_from_user(&mut self, justification: J::Unverified) {
        trace!(
            target: LOG_TARGET,
            "Handling a justification {:?} from user.",
            justification,
        );
        self.metrics
            .report_event(Event::HandleJustificationFromUser);
        match self.handler.handle_justification_from_user(justification) {
            Ok(true) => self.try_request_chain_extension(),
            Ok(false) => {}
            Err(e) => {
                self.metrics
                    .report_event_error(Event::HandleJustificationFromUser);
                match e {
                    HandlerError::JustificationVerifier(e) => debug!(
                        target: LOG_TARGET,
                        "Could not verify justification from user: {}", e
                    ),
                    HandlerError::HeaderVerifier(e) => debug!(
                        target: LOG_TARGET,
                        "Could not verify header from user: {}", e
                    ),
                    e => warn!(
                        target: LOG_TARGET,
                        "Failed to handle justification from user: {}", e
                    ),
                }
            }
        }
    }

    fn handle_request_response(&mut self, response_items: ResponseItems<B, J>, peer: N::PeerId) {
        trace!(
            target: LOG_TARGET,
            "Handling request response from peer {:?}. Items: {:?}.",
            peer,
            response_items,
        );
        self.metrics.report_event(Event::HandleRequestResponse);
        let (new_info, equivocation_proofs, maybe_error) = self
            .handler
            .handle_request_response(response_items, peer.clone());
        match maybe_error {
            Some(HandlerError::JustificationVerifier(e)) => {
                debug!(
                    target: LOG_TARGET,
                    "Could not verify justification from user: {}", e
                )
            }
            Some(HandlerError::HeaderVerifier(e)) => {
                debug!(
                    target: LOG_TARGET,
                    "Could not verify header from user: {}", e
                )
            }
            Some(e) => warn!(
                target: LOG_TARGET,
                "Failed to handle sync request response from {:?}: {}.", peer, e
            ),
            _ => {}
        }
        self.process_equivocation_proofs(equivocation_proofs);
        if new_info {
            self.try_request_chain_extension();
        }
    }

    fn send_big_response(
        &mut self,
        response_items: &[ResponseItem<B, J>],
        peer: N::PeerId,
    ) -> Result<(), MsgLimiterError> {
        let mut limiter = MsgLimiter::new(response_items);
        while let Some(chunk) = limiter.next_largest_msg()? {
            self.send_to(NetworkData::RequestResponse(chunk.to_vec()), peer.clone())
        }
        Ok(())
    }

    fn handle_request(&mut self, request: Request<J>, peer: N::PeerId) {
        trace!(
            target: LOG_TARGET,
            "Handling a request {:?} from {:?}.",
            request,
            peer
        );
        self.metrics.report_event(Event::HandleRequest);

        match self.handler.handle_request(request) {
            Ok((action, maybe_equivocation_proof)) => {
                self.process_equivocation_proofs(maybe_equivocation_proof);
                match action {
                    Action::Response(response_items) => {
                        if let Err(e) = self.send_big_response(&response_items, peer) {
                            error!(
                                target: LOG_TARGET,
                                "Error while sending request response: {}.", e
                            );
                            self.metrics.report_event_error(Event::HandleRequest);
                        }
                    }
                    Action::RequestBlock(header) => self.request_block(header.id()),
                    _ => {}
                }
            }
            Err(e) => {
                self.metrics.report_event_error(Event::HandleRequest);
                match e {
                    HandlerError::JustificationVerifier(e) => debug!(
                        target: LOG_TARGET,
                        "Could not verify justification from user: {}", e
                    ),
                    HandlerError::HeaderVerifier(e) => debug!(
                        target: LOG_TARGET,
                        "Could not verify header from user: {}", e
                    ),
                    e => warn!(
                        target: LOG_TARGET,
                        "Error handling request from {:?}: {}.", peer, e
                    ),
                }
            }
        }
    }

    fn handle_task(&mut self, task: RequestTask) {
        trace!(target: LOG_TARGET, "Handling task {}.", task);
        if let TaskAction::Request(pre_request, (task, delay)) =
            task.process(self.handler.interest_provider())
        {
            self.send_request(pre_request);
            self.tasks.schedule_in(task, delay);
        }
        self.metrics.report_event(Event::HandleTask);
    }

    fn handle_chain_event(&mut self, event: ChainStatusNotification<J::Header>) {
        use ChainStatusNotification::*;
        match event {
            BlockImported(header) => {
                trace!(target: LOG_TARGET, "Handling a new imported block.");
                self.metrics.report_event(Event::HandleBlockImported);
                if let Err(e) = self.handler.block_imported(header) {
                    self.metrics.report_event_error(Event::HandleBlockImported);
                    error!(
                        target: LOG_TARGET,
                        "Error marking block as imported: {}.", e
                    )
                }
            }
            BlockFinalized(_) => {
                trace!(target: LOG_TARGET, "Handling a new finalized block.");
                self.metrics.report_event(Event::HandleBlockFinalized);
            }
        }
        // We either learned about a new finalized or best block, so we
        // might want to broadcast. This will also fire whenever we import
        // forks, but that is rare and mostly harmless.
        if self.broadcast_ticker.try_tick() {
            self.broadcast();
        }
    }

    fn handle_internal_request(&mut self, header: B::UnverifiedHeader) {
        let id = header.id();
        trace!(
            target: LOG_TARGET,
            "Handling an internal request for block {:?}.",
            id,
        );
        self.metrics.report_event(Event::HandleInternalRequest);
        match self.handler.handle_internal_request(header) {
            Ok((request, maybe_equivocation)) => {
                if request {
                    self.request_block(id);
                }
                self.process_equivocation_proofs(maybe_equivocation);
            }
            Err(e) => {
                self.metrics.report_event(Event::HandleInternalRequest);
                match e {
                    HandlerError::HeaderVerifier(e) => debug!(
                        target: LOG_TARGET,
                        "Could not verify header from user: {}", e
                    ),
                    e => warn!(
                        target: LOG_TARGET,
                        "Error handling internal request for block {:?}: {}.", id, e
                    ),
                }
            }
        }
    }

    fn handle_legacy_internal_request(&mut self, id: BlockId) {
        trace!(
            target: LOG_TARGET,
            "Handling a legacy internal request for block {:?}.",
            id,
        );
        self.metrics.report_event(Event::HandleInternalRequest);
        match self.handler.handle_legacy_internal_request(&id) {
            Ok(true) => self.request_block(id),

            Ok(_) => debug!(target: LOG_TARGET, "Already requested block {:?}.", id),

            Err(e) => {
                self.metrics.report_event(Event::HandleInternalRequest);
                warn!(
                    target: LOG_TARGET,
                    "Error handling legacy internal request for block {:?}: {}.", id, e
                )
            }
        }
    }

    fn handle_chain_extension_request(&mut self, state: State<J>, peer: N::PeerId) {
        self.metrics.report_event(Event::HandleExtensionRequest);
        match self.handler.handle_chain_extension_request(state) {
            Ok(Action::Response(response_items)) => {
                if let Err(e) = self.send_big_response(&response_items, peer) {
                    error!(
                        target: LOG_TARGET,
                        "Error while sending chain extension request response: {}.", e
                    );
                    self.metrics
                        .report_event_error(Event::HandleExtensionRequest);
                }
            }
            Ok(Action::RequestBlock(header)) => self.request_block(header.id()),
            Ok(Action::Noop) => {}
            Err(e) => {
                self.metrics
                    .report_event_error(Event::HandleExtensionRequest);
                match e {
                    HandlerError::JustificationVerifier(e) => debug!(
                        target: LOG_TARGET,
                        "Could not verify justification from {:?}: {}", peer, e
                    ),
                    HandlerError::HeaderVerifier(e) => debug!(
                        target: LOG_TARGET,
                        "Could not verify header from {:?}: {}", peer, e
                    ),
                    e => warn!(
                        target: LOG_TARGET,
                        "Error handling chain extension request from {:?}: {}.", peer, e
                    ),
                }
            }
        }
    }

    fn handle_network_data(&mut self, data: NetworkData<B, J>, peer: N::PeerId) {
        use NetworkData::*;
        match data {
            StateBroadcast(state) => self.handle_state(state, peer),
            StateBroadcastResponse(justification, maybe_justification) => {
                self.handle_state_response(justification, maybe_justification, peer)
            }
            Request(request) => {
                let state = request.state().clone();
                self.handle_request(request, peer.clone());
                self.handle_state(state, peer);
            }
            RequestResponse(response_items) => self.handle_request_response(response_items, peer),
            ChainExtensionRequest(state) => self.handle_chain_extension_request(state, peer),
        }
    }

    fn handle_own_block(&mut self, block: B) {
        match self.handler.handle_own_block(block) {
            Ok((broadcast, maybe_proof)) => {
                self.process_equivocation_proofs(maybe_proof);
                if let Err(e) = self
                    .network
                    .broadcast(NetworkData::RequestResponse(broadcast))
                {
                    warn!(
                        target: LOG_TARGET,
                        "Error broadcasting newly created block: {}.", e
                    )
                };
            }
            Err(e) => {
                warn!(
                    target: LOG_TARGET,
                    "Error handling newly created block: {}.", e
                );
            }
        };
    }

    /// Stay synchronized.
    pub async fn run(mut self) {
        loop {
            tokio::select! {
                maybe_data = self.network.next() => match maybe_data {
                    Ok((data, peer)) => self.handle_network_data(data, peer),
                    Err(e) => warn!(target: LOG_TARGET, "Error receiving data from network: {}.", e),
                },
                Some(task) = self.tasks.pop() => self.handle_task(task),
                _ = self.broadcast_ticker.wait_and_tick() => self.broadcast(),
                force = self.chain_extension_ticker.wait_and_tick() => self.request_chain_extension(force),
                maybe_event = self.chain_events.next() => match maybe_event {
                    Ok(chain_event) => self.handle_chain_event(chain_event),
                    Err(e) => warn!(target: LOG_TARGET, "Error when receiving a chain event: {}.", e),
                },
                maybe_justification = self.justifications_from_user.next() => match maybe_justification {
                    Some(justification) => {
                        debug!(target: LOG_TARGET, "Received new justification from user: {:?}.", justification);
                        self.handle_justification_from_user(justification);
                    },
                    None => warn!(target: LOG_TARGET, "Channel with justifications from user closed."),
                },
                maybe_justification = self.additional_justifications_from_user.next() => match maybe_justification {
                    Some(justification) => {
                        debug!(target: LOG_TARGET, "Received new additional justification from user: {:?}.", justification);
                        self.handle_justification_from_user(justification);
                    },
                    None => warn!(target: LOG_TARGET, "Channel with additional justifications from user closed."),
                },
                maybe_header = self.block_requests_from_user.next() => match maybe_header {
                    Some(header) => {
                        debug!(target: LOG_TARGET, "Received new internal block request from user: {:?}.", header);
                        self.handle_internal_request(header)
                    },
                    None => warn!(target: LOG_TARGET, "Channel with internal block request from user closed."),
                },
                maybe_block_id = self.legacy_block_requests_from_user.next() => match maybe_block_id {
                    Some(block_id) => {
                        debug!(target: LOG_TARGET, "Received new internal block request from user: {:?}.", block_id);
                        self.handle_legacy_internal_request(block_id)
                    },
                    None => warn!(target: LOG_TARGET, "Channel with legacy internal block request from user closed."),
                },
                maybe_own_block = self.blocks_from_creator.next() => match maybe_own_block {
                    Some(block) => {
                        debug!(target: LOG_TARGET, "Received new own block: {:?}.", block.header().id());
                        self.handle_own_block(block)
                    },
                    None => warn!(target: LOG_TARGET, "Channel with own blocks closed."),
                },
            }
        }
    }
}
