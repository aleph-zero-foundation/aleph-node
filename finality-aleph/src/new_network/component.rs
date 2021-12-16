use crate::new_network::{DataNetwork, SendError};
use aleph_bft::Recipient;
use codec::Codec;
use futures::channel::mpsc;
use std::sync::Arc;
use tokio::sync::Mutex;

/// For sending arbitrary messages.
pub trait Sender<D: Clone + Codec + Send>: Clone {
    fn send(&self, data: D, recipient: Recipient) -> Result<(), SendError>;
}

/// For receiving arbitrary messages.
#[async_trait::async_trait]
pub trait Receiver<D: Clone + Codec + Send>: Sync + Send {
    async fn next(&mut self) -> Option<D>;
}

/// A bare version of network components.
pub trait Network<D: Clone + Codec + Send>: Sync {
    type S: Sender<D>;
    type R: Receiver<D>;
    fn sender(&self) -> &Self::S;
    fn receiver(&self) -> Arc<Mutex<Self::R>>;
}

#[async_trait::async_trait]
impl<D: Clone + Codec + Send, CN: Network<D>> DataNetwork<D> for CN {
    fn send(&self, data: D, recipient: Recipient) -> Result<(), SendError> {
        self.sender().send(data, recipient)
    }
    async fn next(&self) -> Option<D> {
        self.receiver().clone().lock_owned().await.next().await
    }
}

#[async_trait::async_trait]
impl<D: Clone + Codec + Send> Receiver<D> for mpsc::UnboundedReceiver<D> {
    async fn next(&mut self) -> Option<D> {
        self.next().await
    }
}
