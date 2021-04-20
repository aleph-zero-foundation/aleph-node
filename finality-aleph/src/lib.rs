use sp_keystore::{SyncCryptoStore, SyncCryptoStorePtr};

use codec::{Decode, Encode};
use futures::Future;
pub use rush::Config as ConsensusConfig;
use rush::{nodes::NodeIndex, UnitCoord};
use sc_client_api::{
    backend::{AuxStore, Backend},
    BlockchainEvents, ExecutorProvider, Finalizer, LockImportRun, TransactionFor,
};
use sc_service::SpawnTaskHandle;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::{HeaderBackend, HeaderMetadata};
use sp_consensus::{BlockImport, SelectChain};
use sp_runtime::traits::Block;
use std::{
    convert::TryInto,
    fmt::{Debug, Display},
    sync::Arc,
};
pub mod config;
pub(crate) mod environment;
pub mod hash;
mod import;
mod messages;
mod network;
mod party;

pub use import::AlephBlockImport;

// NOTE until we have our own pallet, we need to use Aura authorities
// mod key_types {
//     use sp_runtime::KeyTypeId;

//     pub const ALEPH: KeyTypeId = KeyTypeId(*b"alph");
// }

// mod app {
//     use crate::key_types::ALEPH;
//     use sp_application_crypto::{app_crypto, ed25519};
//     app_crypto!(ed25519, ALEPH);
// }

// pub type AuthorityId = app::Public;
// pub type AuthoritySignature = app::Signature;
// pub type AuthorityPair = app::Pair;

pub fn peers_set_config() -> sc_network::config::NonDefaultSetConfig {
    sc_network::config::NonDefaultSetConfig {
        notifications_protocol: network::ALEPH_PROTOCOL_NAME.into(),
        max_notification_size: 1024 * 1024,
        set_config: sc_network::config::SetConfig {
            in_peers: 0,
            out_peers: 0,
            reserved_nodes: vec![],
            non_reserved_mode: sc_network::config::NonReservedPeerMode::Accept,
        },
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode)]
pub struct EpochId(pub u32);

use sp_core::crypto::KeyTypeId;
// pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"alp0");
pub const KEY_TYPE: KeyTypeId = sp_application_crypto::key_types::AURA;
pub use sp_consensus_aura::sr25519::{AuthorityId, AuthorityPair, AuthoritySignature};

#[derive(Clone, Debug, Default, Eq, Hash, Encode, Decode, PartialEq)]
pub struct NodeId {
    pub auth: AuthorityId,
    pub index: NodeIndex,
}

impl Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        Display::fmt(&self.auth, f)
    }
}

impl rush::MyIndex for NodeId {
    fn my_index(&self) -> Option<NodeIndex> {
        Some(self.index)
    }
}

/// Ties an authority identification and a cryptography keystore together for use in
/// signing that requires an authority.
#[derive(Clone)]
pub struct AuthorityKeystore {
    key_type_id: KeyTypeId,
    authority_id: AuthorityId,
    keystore: SyncCryptoStorePtr,
}

impl AuthorityKeystore {
    /// Constructs a new authority cryptography keystore.
    pub fn new(authority_id: AuthorityId, keystore: SyncCryptoStorePtr) -> Self {
        AuthorityKeystore {
            key_type_id: KEY_TYPE,
            authority_id,
            keystore,
        }
    }

    /// Returns a references to the authority id.
    pub fn authority_id(&self) -> &AuthorityId {
        &self.authority_id
    }

    /// Returns a reference to the cryptography keystore.
    pub fn keystore(&self) -> &SyncCryptoStorePtr {
        &self.keystore
    }

    pub fn sign(&self, msg: &[u8]) -> AuthoritySignature {
        SyncCryptoStore::sign_with(
            &*self.keystore,
            self.key_type_id,
            &self.authority_id.clone().into(),
            msg,
        )
        .expect("key is in store")
        .try_into()
        .ok()
        .unwrap()
    }
}

pub trait ClientForAleph<B, BE>:
    LockImportRun<B, BE>
    + Finalizer<B, BE>
    + AuxStore
    + BlockchainEvents<B>
    + ProvideRuntimeApi<B>
    + ExecutorProvider<B>
    + BlockImport<B, Transaction = TransactionFor<BE, B>, Error = sp_consensus::Error>
    + HeaderBackend<B>
    + HeaderMetadata<B, Error = sp_blockchain::Error>
where
    BE: Backend<B>,
    B: Block,
{
}

impl<B, BE, T> ClientForAleph<B, BE> for T
where
    BE: Backend<B>,
    B: Block,
    T: LockImportRun<B, BE>
        + Finalizer<B, BE>
        + AuxStore
        + BlockchainEvents<B>
        + ProvideRuntimeApi<B>
        + ExecutorProvider<B>
        + HeaderBackend<B>
        + HeaderMetadata<B, Error = sp_blockchain::Error>
        + BlockImport<B, Transaction = TransactionFor<BE, B>, Error = sp_consensus::Error>,
{
}

#[derive(Clone)]
struct SpawnHandle(SpawnTaskHandle);

impl From<SpawnTaskHandle> for SpawnHandle {
    fn from(sth: SpawnTaskHandle) -> Self {
        SpawnHandle(sth)
    }
}

impl rush::SpawnHandle for SpawnHandle {
    fn spawn(&self, name: &'static str, task: impl Future<Output = ()> + Send + 'static) {
        self.0.spawn(name, task)
    }
}

pub struct AlephConfig<N, C, SC> {
    pub network: N,
    pub consensus_config: ConsensusConfig<NodeId>,
    pub client: Arc<C>,
    pub select_chain: SC,
    pub spawn_handle: SpawnTaskHandle,
    pub auth_keystore: AuthorityKeystore,
    pub authorities: Vec<AuthorityId>,
}

pub fn run_aleph_consensus<B: Block, BE, C, N, SC>(
    config: AlephConfig<N, C, SC>,
) -> impl Future<Output = ()>
where
    BE: Backend<B> + 'static,
    N: network::Network<B> + 'static,
    C: ClientForAleph<B, BE> + Send + Sync + 'static,
    SC: SelectChain<B> + 'static,
{
    let AlephConfig {
        network,
        consensus_config,
        client,
        select_chain,
        spawn_handle,
        auth_keystore,
        authorities,
    } = config;
    let consensus = party::ConsensusParty::new(client, network, select_chain, auth_keystore);
    async move {
        consensus
            .run(authorities, consensus_config, spawn_handle.into())
            .await;
    }
}
