use crate::{aggregator::SignableHash, crypto::Signature};
use aleph_bft::{rmc::Message, SignatureSet};
use sp_runtime::traits::Block;

pub type NetworkData<B> =
    Message<SignableHash<<B as Block>::Hash>, Signature, SignatureSet<Signature>>;
