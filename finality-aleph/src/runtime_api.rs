use std::{
    fmt::{Display, Formatter},
    marker::PhantomData,
    sync::Arc,
};

use aleph_runtime::SessionKeys;
use frame_support::StorageHasher;
use parity_scale_codec::{Decode, DecodeAll, Encode, Error as DecodeError};
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

    fn access_storage<D: Decode>(
        &self,
        storage_key: Vec<u8>,
        at_block: BlockHash,
    ) -> Result<D, ApiError> {
        let encoded = self
            .client
            .storage(at_block, &sc_client_api::StorageKey(storage_key))
            .map_err(|_| ApiError::StorageAccessFailure)?
            .ok_or(ApiError::NoStorage)?;
        D::decode_all(&mut encoded.0.as_ref()).map_err(ApiError::DecodeError)
    }

    fn read_storage_value<D: Decode>(
        &self,
        pallet: &str,
        item: &str,
        at_block: BlockHash,
    ) -> Result<D, ApiError> {
        let storage_key = [twox_128(pallet.as_bytes()), twox_128(item.as_bytes())].concat();
        match self.access_storage::<D>(storage_key, at_block) {
            Err(ApiError::NoStorage) => Err(ApiError::NoStorageValue(pallet.into(), item.into())),
            other => other,
        }
    }

    #[allow(dead_code)]
    fn read_storage_map<H: StorageHasher, D: Decode, E: Encode>(
        &self,
        pallet: &str,
        item: &str,
        key: E,
        at_block: BlockHash,
    ) -> Result<D, ApiError> {
        let mut storage_key = [twox_128(pallet.as_bytes()), twox_128(item.as_bytes())].concat();
        let hashed_encoded_key = key.using_encoded(H::hash);
        storage_key.extend(hashed_encoded_key.as_ref());
        match self.access_storage::<D>(storage_key, at_block) {
            Err(ApiError::NoStorage) => {
                Err(ApiError::NoStorageMapEntry(pallet.into(), item.into()))
            }
            other => other,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ApiError {
    StorageAccessFailure,
    NoStorage,
    NoStorageMapEntry(String, String),
    NoStorageValue(String, String),
    DecodeError(DecodeError),
}

impl Display for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            ApiError::StorageAccessFailure => {
                write!(f, "blockchain error during a storage read attempt")
            }
            ApiError::NoStorage => write!(f, "no storage found"),
            ApiError::NoStorageMapEntry(pallet, item) => {
                write!(f, "storage map element not found under {}{}", pallet, item)
            }
            ApiError::NoStorageValue(pallet, item) => {
                write!(f, "storage value not found under {}{}", pallet, item)
            }
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

        let queued_keys: QueuedKeys = self.read_storage_value("Session", "QueuedKeys", at)?;
        Ok(queued_keys
            .into_iter()
            .filter_map(|(account_id, keys)| keys.get(AURA).map(|key| (account_id, key)))
            .collect())
    }
}

#[cfg(test)]
mod test {
    use std::{
        collections::{BTreeMap, HashMap},
        sync::Arc,
    };

    use frame_support::Twox64Concat;
    use parity_scale_codec::Encode;
    use primitives::Hash;
    use sp_runtime::Storage;
    use substrate_test_client::ClientExt;

    use super::*;
    use crate::testing::mocks::{TestClientBuilder, TestClientBuilderExt};

    #[tokio::test]
    async fn test_proper_storage_reads() {
        let pallet = twox_128("Pallet".as_bytes());
        let map = twox_128("Map".as_bytes());
        let key1 = Twox64Concat::hash("Key1".encode().as_slice());
        let key2 = Twox64Concat::hash("Key2".encode().as_slice());

        let mut map_path1 = [pallet, map].concat();
        map_path1.extend(key1);
        let mut map_path2 = [pallet, map].concat();
        map_path2.extend(key2);

        let storage_value = twox_128("StorageValue".as_bytes());
        let storage_value_path = [pallet, storage_value].concat();

        let storage = Storage {
            top: BTreeMap::from([
                (map_path1, 1u32.encode()),
                (map_path2, 2u32.encode()),
                (storage_value_path, 3u32.encode()),
            ]),
            children_default: HashMap::new(),
        };

        let mut client_builder = TestClientBuilder::new();
        *client_builder.genesis_init_mut().extra_storage() = storage;
        let client = Arc::new(client_builder.build());
        let genesis_hash = client.genesis_hash();
        let runtime_api = RuntimeApiImpl::new(client);

        let map_value1 = runtime_api.read_storage_map::<Twox64Concat, u32, &str>(
            "Pallet",
            "Map",
            "Key1",
            genesis_hash,
        );
        let map_value2 = runtime_api.read_storage_map::<Twox64Concat, u32, &str>(
            "Pallet",
            "Map",
            "Key2",
            genesis_hash,
        );
        let storage_value =
            runtime_api.read_storage_value::<u32>("Pallet", "StorageValue", genesis_hash);

        assert_eq!(map_value1, Ok(1));
        assert_eq!(map_value2, Ok(2));
        assert_eq!(storage_value, Ok(3));
    }

    #[test]
    fn test_missing_storage() {
        let pallet = twox_128("Pallet".as_bytes());
        let map = twox_128("Map".as_bytes());
        let key1 = Twox64Concat::hash("Key1".encode().as_slice());
        let mut map_path1 = [pallet, map].concat();
        map_path1.extend(key1);

        let storage = Storage {
            top: BTreeMap::from([(map_path1, 1u32.encode())]),
            children_default: HashMap::new(),
        };

        let mut client_builder = TestClientBuilder::new();
        *client_builder.genesis_init_mut().extra_storage() = storage;
        let client = Arc::new(client_builder.build());
        let genesis_hash = client.genesis_hash();
        let runtime_api = RuntimeApiImpl::new(client);

        let result1 = runtime_api.read_storage_map::<Twox64Concat, u32, &str>(
            "Pallet",
            "Map",
            "Key2", // this key doesn't exist in the map
            genesis_hash,
        );
        let result2 = runtime_api.read_storage_value::<u32>("Pallet", "StorageValue", genesis_hash);

        assert_eq!(
            result1,
            Err(ApiError::NoStorageMapEntry("Pallet".into(), "Map".into()))
        );
        assert_eq!(
            result2,
            Err(ApiError::NoStorageValue(
                "Pallet".into(),
                "StorageValue".into()
            ))
        );
    }

    #[test]
    fn test_wrong_data_type_decode_error() {
        let pallet = twox_128("Pallet".as_bytes());
        let map = twox_128("Map".as_bytes());
        let key1 = Twox64Concat::hash("Key1".encode().as_slice());
        let mut map_path1 = [pallet, map].concat();
        map_path1.extend(key1);
        let storage_value = twox_128("StorageValue".as_bytes());
        let storage_value_path = [pallet, storage_value].concat();

        let storage = Storage {
            top: BTreeMap::from([
                (map_path1, 1u32.encode()),
                (storage_value_path, 3u32.encode()),
            ]),
            children_default: HashMap::new(),
        };

        let mut client_builder = TestClientBuilder::new();
        *client_builder.genesis_init_mut().extra_storage() = storage;
        let client = Arc::new(client_builder.build());
        let genesis_hash = client.genesis_hash();
        let runtime_api = RuntimeApiImpl::new(client);

        // parameterize function with String instead of u32
        let result1 = runtime_api.read_storage_map::<Twox64Concat, String, &str>(
            "Pallet",
            "Map",
            "Key1",
            genesis_hash,
        );
        let result2 =
            runtime_api.read_storage_value::<String>("Pallet", "StorageValue", genesis_hash);

        assert!(matches!(result1, Err(ApiError::DecodeError(_))));
        assert!(matches!(result2, Err(ApiError::DecodeError(_))));
    }

    #[test]
    fn test_access_at_nonexistent_block() {
        let pallet = twox_128("Pallet".as_bytes());
        let map = twox_128("Map".as_bytes());
        let key1 = Twox64Concat::hash("Key1".encode().as_slice());
        let mut map_path1 = [pallet, map].concat();
        map_path1.extend(key1);
        let storage_value = twox_128("StorageValue".as_bytes());
        let storage_value_path = [pallet, storage_value].concat();

        let storage = Storage {
            top: BTreeMap::from([
                (map_path1, 1u32.encode()),
                (storage_value_path, 3u32.encode()),
            ]),
            children_default: HashMap::new(),
        };

        let mut client_builder = TestClientBuilder::new();
        *client_builder.genesis_init_mut().extra_storage() = storage;
        let client = Arc::new(client_builder.build());
        let runtime_api = RuntimeApiImpl::new(client);

        let result1 = runtime_api.read_storage_map::<Twox64Concat, u32, &str>(
            "Pallet",
            "Map",
            "Key1",
            Hash::zero(),
        );
        let result2 = runtime_api.read_storage_value::<u32>("Pallet", "StorageValue", Hash::zero());

        assert_eq!(result1, Err(ApiError::StorageAccessFailure));
        assert_eq!(result2, Err(ApiError::StorageAccessFailure));
    }
}
