use std::{
    fmt::{Display, Formatter},
    marker::PhantomData,
    sync::Arc,
};

use aleph_runtime::SessionKeys;
use parity_scale_codec::{Decode, DecodeAll, Error as DecodeError};
use sc_client_api::Backend;
use sp_application_crypto::key_types::AURA;
use sp_core::twox_128;
use sp_runtime::traits::{Block, OpaqueKeys};

use crate::{
    aleph_primitives::{AccountId, AlephSessionApi, AuraId},
    BlockHash, ClientForAleph,
};

/// Trait handling connection between host code and runtime storage
pub trait RuntimeApi: Clone + Send + Sync + 'static {
    type Error: Display;
    /// Returns aura authorities for the next session using state from block `at`
    fn next_aura_authorities(&self, at: BlockHash)
        -> Result<Vec<(AccountId, AuraId)>, Self::Error>;
}

type QueuedKeys = Vec<(AccountId, SessionKeys)>;

pub struct RuntimeApiImpl<C, B, BE>
where
    C: ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: AlephSessionApi<B>,
    B: Block<Hash = BlockHash>,
    BE: Backend<B> + 'static,
{
    client: Arc<C>,
    _phantom: PhantomData<(B, BE)>,
}

impl<C, B, BE> Clone for RuntimeApiImpl<C, B, BE>
where
    C: ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: AlephSessionApi<B>,
    B: Block<Hash = BlockHash>,
    BE: Backend<B> + 'static,
{
    fn clone(&self) -> Self {
        RuntimeApiImpl::new(self.client.clone())
    }
}

impl<C, B, BE> RuntimeApiImpl<C, B, BE>
where
    C: ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: AlephSessionApi<B>,
    B: Block<Hash = BlockHash>,
    BE: Backend<B> + 'static,
{
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _phantom: PhantomData,
        }
    }

    fn read_storage<D: Decode>(
        &self,
        pallet: &str,
        item: &str,
        at_block: BlockHash,
    ) -> Result<D, ApiError> {
        let storage_key = [twox_128(pallet.as_bytes()), twox_128(item.as_bytes())].concat();

        let encoded = match self
            .client
            .storage(at_block, &sc_client_api::StorageKey(storage_key))
        {
            Ok(Some(e)) => e,
            _ => return Err(ApiError::NoStorage(pallet.to_string(), item.to_string())),
        };

        D::decode_all(&mut encoded.0.as_ref()).map_err(ApiError::DecodeError)
    }
}

#[derive(Clone, Debug)]
pub enum ApiError {
    NoStorage(String, String),
    DecodeError(DecodeError),
}

impl Display for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            ApiError::NoStorage(pallet, item) => write!(f, "no storage under {}.{}", pallet, item),
            ApiError::DecodeError(error) => write!(f, "decode error: {:?}", error),
        }
    }
}

impl<C, B, BE> RuntimeApi for RuntimeApiImpl<C, B, BE>
where
    C: ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: AlephSessionApi<B>,
    B: Block<Hash = BlockHash>,
    BE: Backend<B> + 'static,
{
    type Error = ApiError;

    fn next_aura_authorities(
        &self,
        at: BlockHash,
    ) -> Result<Vec<(AccountId, AuraId)>, Self::Error> {
        if let Ok(authorities) = self.client.runtime_api().next_session_aura_authorities(at) {
            return Ok(authorities);
        }

        let queued_keys: QueuedKeys = self.read_storage("Session", "QueuedKeys", at)?;
        Ok(queued_keys
            .into_iter()
            .filter_map(|(account_id, keys)| keys.get(AURA).map(|key| (account_id, key)))
            .collect())
    }
}
