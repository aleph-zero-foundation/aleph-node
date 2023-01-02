use std::fmt::{Display, Error as FmtError, Formatter};

use aleph_primitives::BlockNumber;
use futures::StreamExt;
use sc_client_api::client::{FinalityNotifications, ImportNotifications};
use sp_runtime::traits::{Block as BlockT, Header as SubstrateHeader};
use tokio::select;

use crate::sync::{substrate::BlockId, ChainStatusNotification, ChainStatusNotifier, Header};

/// What can go wrong when waiting for next chain status notification.
#[derive(Debug)]
pub enum Error {
    JustificationStreamClosed,
    ImportStreamClosed,
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
        }
    }
}

/// Substrate specific implementation of `ChainStatusNotifier`.
pub struct SubstrateChainStatusNotifier<B>
where
    B: BlockT,
{
    finality_notifications: FinalityNotifications<B>,
    import_notifications: ImportNotifications<B>,
}

impl<B> SubstrateChainStatusNotifier<B>
where
    B: BlockT,
{
    fn new(
        finality_notifications: FinalityNotifications<B>,
        import_notifications: ImportNotifications<B>,
    ) -> Self {
        Self {
            finality_notifications,
            import_notifications,
        }
    }
}

#[async_trait::async_trait]
impl<B> ChainStatusNotifier<BlockId<B::Header>> for SubstrateChainStatusNotifier<B>
where
    B: BlockT,
    B::Header: SubstrateHeader<Number = BlockNumber>,
{
    type Error = Error;

    async fn next(&mut self) -> Result<ChainStatusNotification<BlockId<B::Header>>, Self::Error> {
        select! {
            maybe_block = self.finality_notifications.next() => {
                maybe_block
                    .map(|block| ChainStatusNotification::BlockFinalized(block.header.id()))
                    .ok_or(Error::JustificationStreamClosed)
            },
            maybe_block = self.import_notifications.next() => {
                maybe_block
                .map(|block| ChainStatusNotification::BlockImported(block.header.id()))
                .ok_or(Error::ImportStreamClosed)
            }
        }
    }
}
