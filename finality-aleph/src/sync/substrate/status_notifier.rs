use std::{
    fmt::{Display, Error as FmtError, Formatter},
    time::{Duration, Instant},
};

use aleph_primitives::BlockNumber;
use futures::StreamExt;
use log::debug;
use sc_client_api::client::{FinalityNotifications, ImportNotifications};
use sp_runtime::traits::{Block as BlockT, Header as SubstrateHeader};
use tokio::{select, time::sleep};

use crate::sync::{
    substrate::chain_status::Error as ChainStatusError, BlockIdentifier, ChainStatus,
    ChainStatusNotification, ChainStatusNotifier, Header, SubstrateChainStatus, LOG_TARGET,
};

/// What can go wrong when waiting for next chain status notification.
#[derive(Debug)]
pub enum Error<B>
where
    B: BlockT,
    B::Header: SubstrateHeader<Number = BlockNumber>,
{
    JustificationStreamClosed,
    ImportStreamClosed,
    ChainStatus(ChainStatusError<B>),
}

impl<B> Display for Error<B>
where
    B: BlockT,
    B::Header: SubstrateHeader<Number = BlockNumber>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use Error::*;
        match self {
            JustificationStreamClosed => {
                write!(f, "finalization notification stream has ended")
            }
            ImportStreamClosed => {
                write!(f, "import notification stream has ended")
            }
            ChainStatus(e) => {
                write!(f, "chain status error: {}", e)
            }
        }
    }
}

/// Substrate specific implementation of `ChainStatusNotifier`. If no blocks are reported through
/// the usual channels for some time it falls back to reading the DB directly and produces
/// notifications that way.
pub struct SubstrateChainStatusNotifier<B>
where
    B: BlockT,
    B::Header: SubstrateHeader<Number = BlockNumber>,
{
    finality_notifications: FinalityNotifications<B>,
    import_notifications: ImportNotifications<B>,
    // The things below here are a hack to ensure all blocks get to the user, even during a major
    // sync. They should almost surely be removed after A0-1760.
    backend: SubstrateChainStatus<B>,
    last_reported: BlockNumber,
    trying_since: Instant,
    catching_up: bool,
}

impl<B> SubstrateChainStatusNotifier<B>
where
    B: BlockT,
    B::Header: SubstrateHeader<Number = BlockNumber>,
{
    pub fn new(
        finality_notifications: FinalityNotifications<B>,
        import_notifications: ImportNotifications<B>,
        backend: SubstrateChainStatus<B>,
    ) -> Result<Self, ChainStatusError<B>> {
        let last_reported = backend.best_block()?.id().number();
        Ok(Self {
            finality_notifications,
            import_notifications,
            backend,
            last_reported,
            trying_since: Instant::now(),
            catching_up: false,
        })
    }

    fn header_at(&self, number: BlockNumber) -> Result<Option<B::Header>, ChainStatusError<B>> {
        match self.backend.hash_for_number(number)? {
            Some(hash) => Ok(self.backend.header_for_hash(hash)?),
            None => Ok(None),
        }
    }
}

#[async_trait::async_trait]
impl<B> ChainStatusNotifier<B::Header> for SubstrateChainStatusNotifier<B>
where
    B: BlockT,
    B::Header: SubstrateHeader<Number = BlockNumber>,
{
    type Error = Error<B>;

    async fn next(&mut self) -> Result<ChainStatusNotification<B::Header>, Self::Error> {
        loop {
            if self.catching_up {
                match self
                    .header_at(self.last_reported + 1)
                    .map_err(Error::ChainStatus)?
                {
                    Some(header) => {
                        self.last_reported += 1;
                        return Ok(ChainStatusNotification::BlockImported(header));
                    }
                    None => {
                        self.catching_up = false;
                        self.trying_since = Instant::now();
                        debug!(
                            target: LOG_TARGET,
                            "Manual reporting caught up, back to normal waiting for imports."
                        );
                    }
                }
            }
            select! {
                maybe_block = self.finality_notifications.next() => {
                    self.trying_since = Instant::now();
                    return maybe_block
                        .map(|block| ChainStatusNotification::BlockFinalized(block.header))
                        .ok_or(Error::JustificationStreamClosed)
                },
                maybe_block = self.import_notifications.next() => {
                    if let Some(block) = &maybe_block {
                        let number = block.header.id().number();
                        if number > self.last_reported {
                            self.last_reported = number;
                        }
                    }
                    self.trying_since = Instant::now();
                    return maybe_block
                        .map(|block| ChainStatusNotification::BlockImported(block.header))
                        .ok_or(Error::ImportStreamClosed)
                },
                _ = sleep((self.trying_since + Duration::from_secs(2)).saturating_duration_since(Instant::now())) => {
                    self.catching_up = true;
                    debug!(target: LOG_TARGET, "No new blocks for 2 seconds, falling back to manual reporting.");
                }
            }
        }
    }
}
