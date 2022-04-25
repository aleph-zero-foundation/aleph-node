use crate::crypto::Signature;
use aleph_bft::SignatureSet;
use aleph_bft_rmc::Message;
use sp_runtime::traits::Block;

mod aggregator;
mod multicast;

pub use aggregator::{BlockSignatureAggregator, IO};
pub use multicast::SignableHash;

pub type RmcNetworkData<B> =
    Message<SignableHash<<B as Block>::Hash>, Signature, SignatureSet<Signature>>;
