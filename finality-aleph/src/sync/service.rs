use core::marker::PhantomData;
use std::{iter, time::Duration};

use futures::{channel::mpsc, StreamExt};
use log::{debug, error, trace, warn};

pub use crate::sync::handler::DatabaseIO;
use crate::{
    network::GossipNetwork,
    session::SessionBoundaryInfo,
    sync::{
        data::{NetworkData, Request, State, VersionWrapper, VersionedNetworkData},
        handler::{
            Error as HandlerError, Handler, SyncAction, MAX_BLOCK_BATCH, MAX_JUSTIFICATION_BATCH,
        },
        task_queue::TaskQueue,
        tasks::{Action as TaskAction, PreRequest, RequestTask},
        ticker::Ticker,
        Block, BlockIdFor, BlockIdentifier, BlockImport, ChainStatus, ChainStatusNotification,
        ChainStatusNotifier, Finalizer, Justification, JustificationSubmissions, RequestBlocks,
        Verifier, LOG_TARGET,
    },
};

const BROADCAST_COOLDOWN: Duration = Duration::from_millis(200);
const BROADCAST_PERIOD: Duration = Duration::from_secs(1);

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
}

impl<J: Justification> JustificationSubmissions<J> for mpsc::UnboundedSender<J::Unverified> {
    type Error = mpsc::TrySendError<J::Unverified>;

    fn submit(&mut self, justification: J::Unverified) -> Result<(), Self::Error> {
        self.unbounded_send(justification)
    }
}

impl<BI: BlockIdentifier> RequestBlocks<BI> for mpsc::UnboundedSender<BI> {
    type Error = mpsc::TrySendError<BI>;

    fn request_block(&mut self, block_id: BI) -> Result<(), Self::Error> {
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
        self.tasks
            .schedule_in(RequestTask::new_highest_justified(block_id), Duration::ZERO);
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
        let data = NetworkData::StateBroadcast(state);
        if let Err(e) = self.network.broadcast(data) {
            warn!(target: LOG_TARGET, "Error sending broadcast: {}.", e);
        }
    }

    fn send_request(&mut self, pre_request: PreRequest<N::PeerId, J>) {
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
        let (request, peers) = pre_request.with_state(state);
        trace!(target: LOG_TARGET, "Sending a request: {:?}", request);
        let data = NetworkData::Request(request);
        if let Err(e) = self.network.send_to_random(data, peers) {
            warn!(target: LOG_TARGET, "Error sending request: {}.", e);
        }
    }

    fn send_to(&mut self, data: NetworkData<B, J>, peer: N::PeerId) {
        if let Err(e) = self.network.send_to(data, peer) {
            warn!(target: LOG_TARGET, "Error sending response: {}.", e);
        }
    }

    fn perform_sync_action(&mut self, action: SyncAction<B, J>, peer: N::PeerId) {
        use SyncAction::*;
        match action {
            Response(data) => self.send_to(data, peer),
            HighestJustified(block_id) => self.request_highest_justified(block_id),
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
        match self.handler.handle_state(state, peer.clone()) {
            Ok(action) => self.perform_sync_action(action, peer),
            Err(e) => match e {
                HandlerError::Verifier(e) => debug!(
                    target: LOG_TARGET,
                    "Could not verify justification in sync state from {:?}: {}.", peer, e
                ),
                e => warn!(
                    target: LOG_TARGET,
                    "Failed to handle sync state from {:?}: {}.", peer, e
                ),
            },
        }
    }

    fn handle_justifications(
        &mut self,
        justifications: Vec<J::Unverified>,
        maybe_peer: Option<N::PeerId>,
    ) {
        if let (Some(peer), true) = (&maybe_peer, justifications.len() > MAX_JUSTIFICATION_BATCH) {
            warn!(
                target: LOG_TARGET,
                "Peer {:?} sent too many justifications: {}, we expected at most {}, aborting.",
                peer,
                justifications.len(),
                MAX_JUSTIFICATION_BATCH,
            );
            return;
        }
        trace!(
            target: LOG_TARGET,
            "Handling {:?} justifications.",
            justifications.len()
        );
        for justification in justifications {
            let maybe_block_id = match self
                .handler
                .handle_justification(justification, maybe_peer.clone())
            {
                Ok(maybe_id) => maybe_id,
                Err(e) => match e {
                    HandlerError::Verifier(e) => {
                        debug!(
                            target: LOG_TARGET,
                            "Could not verify justification from {:?}: {}.",
                            maybe_peer.map_or("user".to_string(), |id| format!("{:?}", id)),
                            e
                        );
                        return;
                    }
                    e => {
                        warn!(
                            target: LOG_TARGET,
                            "Failed to handle justification from {:?}: {}.",
                            maybe_peer.map_or("user".to_string(), |id| format!("{:?}", id)),
                            e
                        );
                        return;
                    }
                },
            };
            if let Some(block_id) = maybe_block_id {
                self.request_highest_justified(block_id);
            }
        }
    }

    fn handle_blocks(&mut self, blocks: Vec<B>, peer: N::PeerId) {
        if blocks.len() > MAX_BLOCK_BATCH {
            warn!(
                target: LOG_TARGET,
                "Peer {:?} sent too many blocks: {}, we expected at most {}, aborting.",
                peer,
                blocks.len(),
                MAX_BLOCK_BATCH,
            );
            return;
        }
        trace!(target: LOG_TARGET, "Handling {:?} blocks.", blocks.len());
        for block in blocks {
            self.handler.handle_block(block);
        }
    }

    fn handle_request(&mut self, request: Request<J>, peer: N::PeerId) {
        trace!(
            target: LOG_TARGET,
            "Handling a request {:?} from {:?}.",
            request,
            peer
        );
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

    fn handle_network_data(&mut self, data: NetworkData<B, J>, peer: N::PeerId) {
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
                self.handle_state(state, peer);
            }
            RequestResponse(blocks, justifications) => {
                self.handle_justifications(justifications, Some(peer.clone()));
                self.handle_blocks(blocks, peer);
            }
        }
    }

    fn handle_task(&mut self, task: RequestTask<BlockIdFor<J>>) {
        trace!(target: LOG_TARGET, "Handling task {}.", task);
        if let TaskAction::Request(pre_request, (task, delay)) = task.process(self.handler.forest())
        {
            self.send_request(pre_request);
            self.tasks.schedule_in(task, delay);
        }
    }

    fn handle_chain_event(&mut self, event: ChainStatusNotification<J::Header>) {
        use ChainStatusNotification::*;
        match event {
            BlockImported(header) => {
                trace!(target: LOG_TARGET, "Handling a new imported block.");
                if let Err(e) = self.handler.block_imported(header) {
                    error!(
                        target: LOG_TARGET,
                        "Error marking block as imported: {}.", e
                    );
                }
            }
            BlockFinalized(_) => {
                trace!(target: LOG_TARGET, "Handling a new finalized block.");
                if self.broadcast_ticker.try_tick() {
                    self.broadcast();
                }
            }
        }
    }

    fn handle_internal_request(&mut self, id: BlockIdFor<J>) {
        trace!(
            target: LOG_TARGET,
            "Handling an internal request for block {:?}.",
            id,
        );
        match self.handler.handle_internal_request(&id) {
            Ok(true) => {
                self.request_block(id);
            }
            Ok(_) => {
                debug!(target: LOG_TARGET, "Already requested block {:?}.", id);
            }
            Err(e) => {
                warn!(
                    target: LOG_TARGET,
                    "Error handling internal request for block {:?}: {}.", id, e
                );
            }
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
