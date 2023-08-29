use std::{fmt::Debug, hash::Hash, num::NonZeroUsize};

use parity_scale_codec::{Decode, Encode};

mod chain_info;
mod data_interpreter;
mod data_provider;
mod data_store;
mod proposal;
mod status_provider;

pub use chain_info::{ChainInfoProvider, SubstrateChainInfoProvider};
pub use data_interpreter::OrderedDataInterpreter;
pub use data_provider::{ChainTracker, DataProvider};
pub use data_store::{DataStore, DataStoreConfig};
pub use proposal::UnvalidatedAlephProposal;

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

#[derive(Clone, Debug)]
pub struct ChainInfoCacheConfig {
    pub block_cache_capacity: NonZeroUsize,
}

impl Default for ChainInfoCacheConfig {
    fn default() -> ChainInfoCacheConfig {
        ChainInfoCacheConfig {
            block_cache_capacity: NonZeroUsize::new(2000).unwrap(),
        }
    }
}
