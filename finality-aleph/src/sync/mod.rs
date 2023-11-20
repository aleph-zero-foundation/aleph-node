use std::{
    fmt::{Debug, Display},
    hash::Hash,
    marker::Send,
};

use crate::{
    block::{Justification, UnverifiedHeader},
    BlockId,
};

mod data;
mod forest;
mod handler;
mod message_limiter;
mod metrics;
mod service;
mod task_queue;
mod tasks;
mod ticker;

pub use handler::DatabaseIO;
pub use service::{Service, IO};

const LOG_TARGET: &str = "aleph-block-sync";

/// The identifier of a connected peer.
pub trait PeerId: Debug + Clone + Hash + Eq {}

impl<T: Debug + Clone + Hash + Eq> PeerId for T {}

/// An interface for submitting additional justifications to the justification sync.
/// Chiefly ones created by ABFT, but others will also be handled appropriately.
/// The block corresponding to the submitted `Justification` MUST be obtained and
/// imported into the Substrate database by the user, as soon as possible.
pub trait JustificationSubmissions<J: Justification>: Clone + Send + 'static {
    type Error: Display;

    /// Submit a justification to the underlying justification sync.
    fn submit(&mut self, justification: J::Unverified) -> Result<(), Self::Error>;
}

/// An interface for requesting specific blocks from the block sync.
/// Required by the data availability mechanism in ABFT.
pub trait RequestBlocks<UH: UnverifiedHeader>: Clone + Send + Sync + 'static {
    type Error: Display;

    /// Request the given block.
    fn request_block(&self, header: UH) -> Result<(), Self::Error>;
}

/// An interface for requesting specific blocks from the block sync.
/// Required by the data availability mechanism in ABFT.
// TODO(A0-3494): Remove this after support for headerless proposals gets dropped.
pub trait LegacyRequestBlocks: Clone + Send + Sync + 'static {
    type Error: Display;

    /// Request the given block.
    fn request_block(&self, block_id: BlockId) -> Result<(), Self::Error>;
}

#[cfg(test)]
pub type MockPeerId = u32;
