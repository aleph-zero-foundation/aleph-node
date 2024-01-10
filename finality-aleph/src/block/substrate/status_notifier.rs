use std::fmt::{Display, Error as FmtError, Formatter};

use futures::{stream::FusedStream, StreamExt};
use sc_client_api::client::{FinalityNotifications, ImportNotifications};

use crate::{
    aleph_primitives::{Block, Header},
    block::{ChainStatusNotification, ChainStatusNotifier},
};

/// What can go wrong when waiting for next chain status notification.
#[derive(Debug)]
pub enum Error {
    JustificationStreamClosed,
    ImportStreamClosed,
    AllStreamsTerminated,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use Error::*;
        match self {
            JustificationStreamClosed => {
                write!(f, "finalization notification stream has ended")
            }
            ImportStreamClosed => {
                write!(f, "import notification stream has ended")
            }
            AllStreamsTerminated => {
                write!(f, "all streams have ended")
            }
        }
    }
}

/// Substrate specific implementation of `ChainStatusNotifier`.
pub struct SubstrateChainStatusNotifier {
    finality_notifications: FinalityNotifications<Block>,
    import_notifications: ImportNotifications<Block>,
}

impl SubstrateChainStatusNotifier {
    pub fn new(
        finality_notifications: FinalityNotifications<Block>,
        import_notifications: ImportNotifications<Block>,
    ) -> Self {
        Self {
            finality_notifications,
            import_notifications,
        }
    }
}

#[async_trait::async_trait]
impl ChainStatusNotifier<Header> for SubstrateChainStatusNotifier {
    type Error = Error;

    async fn next(&mut self) -> Result<ChainStatusNotification<Header>, Self::Error> {
        match (
            self.finality_notifications.is_terminated(),
            self.import_notifications.is_terminated(),
        ) {
            (true, true) => return Err(Error::AllStreamsTerminated),
            (true, _) => return Err(Error::JustificationStreamClosed),
            (_, true) => return Err(Error::ImportStreamClosed),
            _ => {}
        }

        futures::select! {
            maybe_block = self.finality_notifications.next() => {
                maybe_block
                    .map(|block| ChainStatusNotification::BlockFinalized(block.header))
                    .ok_or(Error::JustificationStreamClosed)
            },
            maybe_block = self.import_notifications.next() => {
                maybe_block
                .map(|block| ChainStatusNotification::BlockImported(block.header))
                .ok_or(Error::ImportStreamClosed)
            }
            complete => Err(Error::AllStreamsTerminated)
        }
    }
}
