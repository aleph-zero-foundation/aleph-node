use std::fmt::Debug;

use aleph_bft_rmc::Message as RmcMessage;
use aleph_bft_types::Recipient;

const LOG_TARGET: &str = "aleph-aggregator";

mod aggregator;

pub use crate::aggregator::{HashSignatureAggregator, IO};

pub type RmcNetworkData<H, S, SS> = RmcMessage<H, S, SS>;

#[derive(Debug)]
pub enum NetworkError {
    SendFail,
}

#[async_trait::async_trait]
pub trait ProtocolSink<D>: Send + Sync {
    async fn next(&mut self) -> Option<D>;
    fn send(&self, data: D, recipient: Recipient) -> Result<(), NetworkError>;
}
