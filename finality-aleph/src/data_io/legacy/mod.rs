use std::{fmt::Debug, hash::Hash};

use parity_scale_codec::{Decode, Encode};

mod data_interpreter;
mod data_provider;
mod data_store;
mod proposal;
mod status_provider;

pub use data_interpreter::OrderedDataInterpreter;
pub use data_provider::{ChainTracker, DataProvider};
pub use data_store::{DataStore, DataStoreConfig};
pub use proposal::UnvalidatedAlephProposal;

pub use super::ChainInfoCacheConfig;

// Maximum number of blocks above the last finalized allowed in an AlephBFT proposal.
pub const MAX_DATA_BRANCH_LEN: usize = 7;

/// The data ordered by the Aleph consensus.
#[derive(Clone, Debug, Encode, Decode, Hash, PartialEq, Eq)]
pub struct AlephData {
    pub head_proposal: UnvalidatedAlephProposal,
}

/// A trait allowing to check the data contained in an AlephBFT network message, for the purpose of
/// data availability checks.
pub trait AlephNetworkMessage: Clone + Debug {
    fn included_data(&self) -> Vec<AlephData>;
}

#[cfg(test)]
mod test {
    use crate::{
        data_io::legacy::{AlephData, UnvalidatedAlephProposal},
        testing::mocks::{TBlock, THeader},
    };

    pub fn unvalidated_proposal_from_headers(headers: Vec<THeader>) -> UnvalidatedAlephProposal {
        let num = headers.last().unwrap().number;
        let hashes = headers.into_iter().map(|header| header.hash()).collect();
        UnvalidatedAlephProposal::new(hashes, num)
    }

    pub fn aleph_data_from_blocks(blocks: Vec<TBlock>) -> AlephData {
        let headers = blocks.into_iter().map(|b| b.header).collect();
        aleph_data_from_headers(headers)
    }

    pub fn aleph_data_from_headers(headers: Vec<THeader>) -> AlephData {
        AlephData {
            head_proposal: unvalidated_proposal_from_headers(headers),
        }
    }
}
