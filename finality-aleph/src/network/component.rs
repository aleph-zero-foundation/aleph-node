use std::{marker::PhantomData, sync::Arc};

use aleph_bft::Recipient;
use futures::{channel::mpsc, StreamExt};
use tokio::sync::Mutex;

use crate::network::{Data, DataNetwork, SendError};

/// For sending arbitrary messages.
pub trait Sender<D: Data>: Sync + Send + Clone {
    fn send(&self, data: D, recipient: Recipient) -> Result<(), SendError>;
}

/// For receiving arbitrary messages.
#[async_trait::async_trait]
pub trait Receiver<D: Data>: Sync + Send {
    async fn next(&mut self) -> Option<D>;
}

/// A bare version of network components.
pub trait Network<D: Data>: Sync + Send {
    type S: Sender<D>;
    type R: Receiver<D>;
    fn sender(&self) -> &Self::S;
    fn receiver(&self) -> Arc<Mutex<Self::R>>;
}

#[async_trait::async_trait]
impl<D: Data, CN: Network<D>> DataNetwork<D> for CN {
    fn send(&self, data: D, recipient: Recipient) -> Result<(), SendError> {
        self.sender().send(data, recipient)
    }
    async fn next(&mut self) -> Option<D> {
        self.receiver().lock_owned().await.next().await
    }
}

#[async_trait::async_trait]
impl<D: Data> Sender<D> for mpsc::UnboundedSender<(D, Recipient)> {
    fn send(&self, data: D, recipient: Recipient) -> Result<(), SendError> {
        self.unbounded_send((data, recipient))
            .map_err(|_| SendError::SendFailed)
    }
}

#[async_trait::async_trait]
impl<D: Data> Receiver<D> for mpsc::UnboundedReceiver<D> {
    async fn next(&mut self) -> Option<D> {
        StreamExt::next(self).await
    }
}

pub struct SimpleNetwork<D: Data, R: Receiver<D>, S: Sender<D>> {
    receiver: Arc<Mutex<R>>,
    sender: S,
    _phantom: PhantomData<D>,
}

impl<D: Data, R: Receiver<D>, S: Sender<D>> SimpleNetwork<D, R, S> {
    pub fn new(receiver: R, sender: S) -> Self {
        SimpleNetwork {
            receiver: Arc::new(Mutex::new(receiver)),
            sender,
            _phantom: PhantomData,
        }
    }
}

impl<D: Data, R: Receiver<D>, S: Sender<D>> Network<D> for SimpleNetwork<D, R, S> {
    type S = S;

    type R = R;

    fn sender(&self) -> &Self::S {
        &self.sender
    }

    fn receiver(&self) -> Arc<Mutex<Self::R>> {
        self.receiver.clone()
    }
}

#[cfg(test)]
mod tests {
    use futures::channel::mpsc;

    use super::Receiver;

    #[tokio::test]
    async fn test_receiver_implementation() {
        let (sender, mut receiver) = mpsc::unbounded();

        let val = 1234;
        sender.unbounded_send(val).unwrap();
        let received = Receiver::<u64>::next(&mut receiver).await;
        assert_eq!(Some(val), received);
    }
}
