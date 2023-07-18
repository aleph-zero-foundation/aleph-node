use std::fmt::{Display, Error as FmtError, Formatter};

use futures::channel::mpsc::UnboundedReceiver;

use crate::sync::{
    mock::{MockHeader, MockNotification},
    ChainStatusNotifier,
};

#[derive(Debug)]
pub enum Error {
    StreamClosed,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(f, "{self:?}")
    }
}

#[async_trait::async_trait]
impl ChainStatusNotifier<MockHeader> for UnboundedReceiver<MockNotification> {
    type Error = Error;

    async fn next(&mut self) -> Result<MockNotification, Self::Error> {
        <Self as futures::StreamExt>::next(self)
            .await
            .ok_or(Error::StreamClosed)
    }
}
