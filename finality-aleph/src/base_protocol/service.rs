use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    sync::{
        atomic::{AtomicBool, AtomicUsize},
        Arc,
    },
};

use futures::stream::StreamExt;
use log::{debug, trace, warn};
use sc_network::config::FullNetworkConfiguration;
use sc_network_sync::{service::syncing_service::ToServiceCommand, SyncEvent, SyncingService};
use sc_utils::mpsc::{tracing_unbounded, TracingUnboundedReceiver, TracingUnboundedSender};
use sp_runtime::traits::{Block, Header};

use crate::{
    base_protocol::{handler::Handler, LOG_TARGET},
    BlockHash, BlockNumber,
};

#[derive(Debug)]
pub enum Error {
    NoIncomingCommands,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        use Error::*;
        match self {
            NoIncomingCommands => write!(f, "Channel with commands from user closed."),
        }
    }
}

/// A service that needs to be run to have the base protocol of the network work.
pub struct Service<B>
where
    B: Block<Hash = BlockHash>,
    B::Header: Header<Number = BlockNumber>,
{
    handler: Handler<B>,
    commands_from_user: TracingUnboundedReceiver<ToServiceCommand<B>>,
    events_for_users: Vec<TracingUnboundedSender<SyncEvent>>,
}

impl<B> Service<B>
where
    B: Block<Hash = BlockHash>,
    B::Header: Header<Number = BlockNumber>,
{
    /// Create a new service.
    // TODO(A0-3886): This shouldn't need to return the substrate type after replacing RPCs.
    // In particular, it shouldn't depend on `B`. This is also the only reason why
    // the `major_sync` argument is needed.
    pub fn new(
        major_sync: Arc<AtomicBool>,
        genesis_hash: B::Hash,
        net_config: &FullNetworkConfiguration,
    ) -> (Self, SyncingService<B>) {
        let (commands_for_service, commands_from_user) =
            tracing_unbounded("mpsc_base_protocol", 100_000);
        (
            Service {
                handler: Handler::new(genesis_hash, net_config),
                commands_from_user,
                events_for_users: Vec::new(),
            },
            SyncingService::new(
                commands_for_service,
                // We don't care about this one, so a dummy value.
                Arc::new(AtomicUsize::new(0)),
                major_sync,
            ),
        )
    }

    fn handle_command(&mut self, command: ToServiceCommand<B>) {
        use ToServiceCommand::*;
        match command {
            EventStream(events_for_user) => self.events_for_users.push(events_for_user),
            PeersInfo(_) => {
                debug!(
                    target: LOG_TARGET,
                    "Failed to send response to peers info request - call unsupported."
                );
            }
            BestSeenBlock(response) => {
                if response.send(None).is_err() {
                    debug!(
                        target: LOG_TARGET,
                        "Failed to send response to best block request."
                    );
                }
            }
            Status(_) => {
                // We are explicitly dropping the response channel to cause an `Err(())` to be returned in the interface, as this produces the desired results for us.
                trace!(target: LOG_TARGET, "Got status request, ignoring.");
            }
            _ => {
                warn!(target: LOG_TARGET, "Got unexpected service command.");
            }
        }
    }

    /// Run the service managing the base protocol.
    pub async fn run(mut self) -> Result<(), Error> {
        use Error::*;
        loop {
            let command = self
                .commands_from_user
                .next()
                .await
                .ok_or(NoIncomingCommands)?;
            self.handle_command(command);
        }
    }
}
