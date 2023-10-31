use core::marker::PhantomData;
use std::time::Duration;

use futures::{channel::mpsc, StreamExt};
use log::{debug, error, trace, warn};
use substrate_prometheus_endpoint::Registry;

pub use crate::sync::handler::DatabaseIO;
use crate::{
    network::GossipNetwork,
    session::SessionBoundaryInfo,
    sync::{
        data::{NetworkData, Request, ResponseItems, State, VersionWrapper, VersionedNetworkData},
        handler::{Action, Error as HandlerError, HandleStateAction, Handler},
        message_limiter::MsgLimiter,
        metrics::{Event, Metrics},
        task_queue::TaskQueue,
        tasks::{Action as TaskAction, PreRequest, RequestTask},
        ticker::Ticker,
        Block, BlockIdFor, BlockIdentifier, BlockImport, ChainStatus, ChainStatusNotification,
        ChainStatusNotifier, Finalizer, Header, Justification, JustificationSubmissions,
        RequestBlocks, Verifier, LOG_TARGET,
    },
};

const BROADCAST_COOLDOWN: Duration = Duration::from_millis(600);
const BROADCAST_PERIOD: Duration = Duration::from_secs(5);

/// A service synchronizing the knowledge about the chain between the nodes.
pub struct Service<B, J, N, CE, CS, V, F, BI>
where
    B: Block,
    J: Justification<Header = B::Header>,
    N: GossipNetwork<VersionedNetworkData<B, J>>,
    CE: ChainStatusNotifier<B::Header>,
    CS: ChainStatus<B, J>,
    V: Verifier<J>,
    F: Finalizer<J>,
    BI: BlockImport<B>,
{
    network: VersionWrapper<B, J, N>,
    handler: Handler<B, N::PeerId, J, CS, V, F, BI>,
    tasks: TaskQueue<RequestTask<BlockIdFor<J>>>,
    broadcast_ticker: Ticker,
    chain_events: CE,
    justifications_from_user: mpsc::UnboundedReceiver<J::Unverified>,
    additional_justifications_from_user: mpsc::UnboundedReceiver<J::Unverified>,
    block_requests_from_user: mpsc::UnboundedReceiver<BlockIdFor<J>>,
    _phantom: PhantomData<B>,
    metrics: Metrics,
}

impl<J: Justification> JustificationSubmissions<J> for mpsc::UnboundedSender<J::Unverified> {
    type Error = mpsc::TrySendError<J::Unverified>;

    fn submit(&mut self, justification: J::Unverified) -> Result<(), Self::Error> {
        self.unbounded_send(justification)
    }
}

impl<BI: BlockIdentifier> RequestBlocks<BI> for mpsc::UnboundedSender<BI> {
    type Error = mpsc::TrySendError<BI>;

    fn request_block(&self, block_id: BI) -> Result<(), Self::Error> {
        self.unbounded_send(block_id)
    }
}

impl<B, J, N, CE, CS, V, F, BI> Service<B, J, N, CE, CS, V, F, BI>
where
    B: Block,
    J: Justification<Header = B::Header>,
    N: GossipNetwork<VersionedNetworkData<B, J>>,
    CE: ChainStatusNotifier<B::Header>,
    CS: ChainStatus<B, J>,
    V: Verifier<J>,
    F: Finalizer<J>,
    BI: BlockImport<B>,
{
    /// Create a new service using the provided network for communication.
    /// Also returns an interface for submitting additional justifications,
    /// and an interface for requesting blocks.
    pub fn new(
        network: N,
        chain_events: CE,
        verifier: V,
        database_io: DatabaseIO<B, J, CS, F, BI>,
        session_info: SessionBoundaryInfo,
        additional_justifications_from_user: mpsc::UnboundedReceiver<J::Unverified>,
        metrics_registry: Option<Registry>,
    ) -> Result<
        (
            Self,
            impl JustificationSubmissions<J> + Clone,
            impl RequestBlocks<BlockIdFor<J>>,
        ),
        HandlerError<B, J, CS, V, F>,
    > {
        let network = VersionWrapper::new(network);
        let handler = Handler::new(database_io, verifier, session_info)?;
        let tasks = TaskQueue::new();
        let broadcast_ticker = Ticker::new(BROADCAST_PERIOD, BROADCAST_COOLDOWN);
        let (justifications_for_sync, justifications_from_user) = mpsc::unbounded();
        let (block_requests_for_sync, block_requests_from_user) = mpsc::unbounded();
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
                chain_events,
                justifications_from_user,
                additional_justifications_from_user,
                block_requests_from_user,
                metrics,
                _phantom: PhantomData,
            },
            justifications_for_sync,
            block_requests_for_sync,
        ))
    }

    fn request_highest_justified(&mut self, block_id: BlockIdFor<J>) {
        debug!(
            target: LOG_TARGET,
            "Initiating a request for highest justified block {:?}.", block_id
        );
        self.tasks.schedule_in(
            RequestTask::new_highest_justified(block_id),
            Duration::from_millis(200),
        );
    }

    fn request_block(&mut self, block_id: BlockIdFor<J>) {
        debug!(
            target: LOG_TARGET,
            "Initiating a request for block {:?}.", block_id
        );
        self.tasks
            .schedule_in(RequestTask::new_block(block_id), Duration::ZERO);
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

    fn send_request(&mut self, pre_request: PreRequest<N::PeerId, J>) {
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
            Ok(action) => match action {
                Response(data) => self.send_to(data, peer),
                HighestJustified(block_id) => self.request_highest_justified(block_id),
                Noop => (),
            },
            Err(e) => {
                self.metrics.report_event_error(Event::HandleState);
                match e {
                    HandlerError::Verifier(e) => debug!(
                        target: LOG_TARGET,
                        "Could not verify justification in sync state from {:?}: {}.", peer, e
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
        let (maybe_id, maybe_error) =
            self.handler
                .handle_state_response(justification, maybe_justification, peer.clone());
        match maybe_error {
            Some(HandlerError::Verifier(e)) => debug!(
                target: LOG_TARGET,
                "Could not verify justification in sync state from {:?}: {}.", peer, e
            ),
            Some(e) => warn!(
                target: LOG_TARGET,
                "Failed to handle sync state response from {:?}: {}.", peer, e
            ),
            _ => {}
        }
        if let Some(id) = maybe_id {
            self.request_highest_justified(id);
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
            Ok(Some(id)) => self.request_highest_justified(id),
            Ok(_) => {}
            Err(e) => {
                self.metrics
                    .report_event_error(Event::HandleJustificationFromUser);
                match e {
                    HandlerError::Verifier(e) => debug!(
                        target: LOG_TARGET,
                        "Could not verify justification from user: {}", e
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
        let (maybe_id, maybe_error) = self
            .handler
            .handle_request_response(response_items, peer.clone());
        match maybe_error {
            Some(HandlerError::Verifier(e)) => debug!(
                target: LOG_TARGET,
                "Could not verify justification from user: {}", e
            ),
            Some(e) => warn!(
                target: LOG_TARGET,
                "Failed to handle sync request response from {:?}: {}.", peer, e
            ),
            _ => {}
        }
        if let Some(id) = maybe_id {
            self.request_highest_justified(id);
        }
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
            Ok(Action::Response(response_items)) => {
                let mut limiter = MsgLimiter::new(&response_items);
                loop {
                    match limiter.next_largest_msg() {
                        Ok(None) => {
                            break;
                        }
                        Ok(Some(chunk)) => {
                            self.send_to(NetworkData::RequestResponse(chunk.to_vec()), peer.clone())
                        }
                        Err(e) => {
                            error!(
                                target: LOG_TARGET,
                                "Error while sending request response: {}.", e
                            );
                            break self.metrics.report_event_error(Event::HandleRequest);
                        }
                    }
                }
            }
            Ok(Action::RequestBlock(id)) => self.request_block(id),
            Err(e) => {
                self.metrics.report_event_error(Event::HandleRequest);
                match e {
                    HandlerError::Verifier(e) => debug!(
                        target: LOG_TARGET,
                        "Could not verify justification from user: {}", e
                    ),
                    e => warn!(
                        target: LOG_TARGET,
                        "Error handling request from {:?}: {}.", peer, e
                    ),
                }
            }
            _ => {}
        }
    }

    fn handle_task(&mut self, task: RequestTask<BlockIdFor<J>>) {
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
                let number = header.id().number();
                trace!(target: LOG_TARGET, "Handling a new imported block.");
                self.metrics.report_event(Event::HandleBlockImported);
                if let Err(e) = self.handler.block_imported(header) {
                    self.metrics.report_event_error(Event::HandleBlockImported);
                    error!(
                        target: LOG_TARGET,
                        "Error marking block as imported: {}.", e
                    )
                } else {
                    // TODO: best block could decrease in case of reorgs.
                    // TODO: use instead is_best_block info from Forest
                    self.metrics.update_best_block_if_better(number);
                }
            }
            BlockFinalized(header) => {
                trace!(target: LOG_TARGET, "Handling a new finalized block.");
                self.metrics.report_event(Event::HandleBlockFinalized);
                if self.broadcast_ticker.try_tick() {
                    self.broadcast();
                }
                self.metrics
                    .update_top_finalized_block(header.id().number());
            }
        }
    }

    fn handle_internal_request(&mut self, id: BlockIdFor<J>) {
        trace!(
            target: LOG_TARGET,
            "Handling an internal request for block {:?}.",
            id,
        );
        self.metrics.report_event(Event::HandleInternalRequest);
        match self.handler.handle_internal_request(&id) {
            Ok(true) => self.request_block(id),

            Ok(_) => debug!(target: LOG_TARGET, "Already requested block {:?}.", id),

            Err(e) => {
                self.metrics.report_event(Event::HandleInternalRequest);
                match e {
                    HandlerError::Verifier(e) => debug!(
                        target: LOG_TARGET,
                        "Could not verify justification from user: {}", e
                    ),
                    e => warn!(
                        target: LOG_TARGET,
                        "Error handling internal request for block {:?}: {}.", id, e
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
        }
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
                maybe_block_id = self.block_requests_from_user.next() => match maybe_block_id {
                    Some(block_id) => {
                        debug!(target: LOG_TARGET, "Received new internal block request from user: {:?}.", block_id);
                        self.handle_internal_request(block_id)
                    },
                    None => warn!(target: LOG_TARGET, "Channel with internal block request from user closed."),
                },
            }
        }
    }
}
