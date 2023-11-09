use std::{collections::HashMap, fmt::Debug, num::NonZeroUsize, sync::Arc};

use lru::LruCache;
use parking_lot::Mutex;
use primitives::AccountId;
use serde::{Deserialize, Serialize};

use crate::{
    abft::NodeIndex, idx_to_account::ValidatorIndexToAccountIdConverter, session::SessionId,
};

/// Network details for a given validator in a given session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorAddressingInfo {
    /// Session to which given information applies.
    pub session: SessionId,
    /// Network level address of the validator, i.e. IP address (for validator network)
    pub network_level_address: String,
    /// PeerId of the validator used in validator (clique) network
    pub validator_network_peer_id: String,
}

/// Stores most recent information about validator addresses.
#[derive(Clone)]
pub struct ValidatorAddressCache {
    data: Arc<Mutex<LruCache<AccountId, ValidatorAddressingInfo>>>,
}

const VALIDATOR_ADDRESS_CACHE_SIZE: usize = 1000;

impl ValidatorAddressCache {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::try_from(VALIDATOR_ADDRESS_CACHE_SIZE)
                    .expect("the cache size is a non-zero constant"),
            ))),
        }
    }

    pub fn insert(&self, validator_stash: AccountId, info: ValidatorAddressingInfo) {
        self.data.lock().put(validator_stash, info);
    }

    pub fn snapshot(&self) -> HashMap<AccountId, ValidatorAddressingInfo> {
        HashMap::from_iter(self.data.lock().iter().map(|(k, v)| (k.clone(), v.clone())))
    }
}

impl Default for ValidatorAddressCache {
    fn default() -> Self {
        Self::new()
    }
}

pub trait ValidatorAddressCacheUpdater {
    /// In session `session_info.session`, validator `NodeIndex` was using addresses specified in
    /// `session_info`. A session and validator_index identify the validator uniquely.
    fn update(&self, validator_index: NodeIndex, session_info: ValidatorAddressingInfo);
}

enum ValidatorAddressCacheUpdaterImpl<C: ValidatorIndexToAccountIdConverter> {
    Noop,
    BackendBased {
        validator_address_cache: ValidatorAddressCache,
        key_owner_info_provider: C,
    },
}

/// Construct a struct that can be used to update `validator_address_cache`, if it is `Some`.
/// If passed None, the returned struct will be a no-op.
pub fn validator_address_cache_updater<C: ValidatorIndexToAccountIdConverter>(
    validator_address_cache: Option<ValidatorAddressCache>,
    key_owner_info_provider: C,
) -> impl ValidatorAddressCacheUpdater {
    match validator_address_cache {
        Some(validator_address_cache) => ValidatorAddressCacheUpdaterImpl::BackendBased {
            validator_address_cache,
            key_owner_info_provider,
        },
        None => ValidatorAddressCacheUpdaterImpl::Noop,
    }
}

impl<C: ValidatorIndexToAccountIdConverter> ValidatorAddressCacheUpdater
    for ValidatorAddressCacheUpdaterImpl<C>
{
    fn update(&self, validator_index: NodeIndex, info: ValidatorAddressingInfo) {
        if let ValidatorAddressCacheUpdaterImpl::BackendBased {
            validator_address_cache,
            key_owner_info_provider,
        } = self
        {
            if let Some(validator) = key_owner_info_provider.account(info.session, validator_index)
            {
                validator_address_cache.insert(validator, info)
            }
        }
    }
}

#[cfg(test)]
pub mod test {
    use crate::{
        idx_to_account::MockConverter,
        network::address_cache::{ValidatorAddressCacheUpdater, ValidatorAddressCacheUpdaterImpl},
    };

    pub fn noop_updater() -> impl ValidatorAddressCacheUpdater {
        ValidatorAddressCacheUpdaterImpl::<MockConverter>::Noop
    }
}
