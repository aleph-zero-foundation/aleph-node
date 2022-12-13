use std::{fmt::Debug, hash::Hash};

use aleph_bft_rmc::Message;
use aleph_bft_types::Recipient;

mod aggregator;
mod multicast;

pub use crate::{
    aggregator::{BlockSignatureAggregator, IO},
    multicast::SignableHash,
};

pub type RmcNetworkData<H, S, SS> = Message<SignableHash<H>, S, SS>;

#[derive(Debug)]
pub enum NetworkError {
    SendFail,
}

#[async_trait::async_trait]
pub trait ProtocolSink<D>: Send + Sync {
    async fn next(&mut self) -> Option<D>;
    fn send(&self, data: D, recipient: Recipient) -> Result<(), NetworkError>;
}

pub trait Metrics<H: Debug + Hash + Eq + Debug + Copy> {
    fn report_aggregation_complete(&mut self, h: H);
}
