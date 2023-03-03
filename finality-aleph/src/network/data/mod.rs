//! Abstraction over an abstract network sending data to a set of nodes.
use crate::{abft::Recipient, network::Data};

pub mod component;
pub mod split;

/// Returned when something went wrong when sending data using a Network.
#[derive(Debug)]
pub enum SendError {
    SendFailed,
}

/// A generic interface for sending and receiving data.
#[async_trait::async_trait]
pub trait Network<D: Data>: Send + Sync {
    fn send(&self, data: D, recipient: Recipient) -> Result<(), SendError>;
    async fn next(&mut self) -> Option<D>;
}
