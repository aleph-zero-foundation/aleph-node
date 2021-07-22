use sp_keystore::{SyncCryptoStore, SyncCryptoStorePtr};

use codec::{Decode, Encode};

pub use aleph_bft::{
    default_config as default_aleph_config, Config as ConsensusConfig, TaskHandle,
};
use aleph_bft::{DefaultMultiKeychain, NodeCount, NodeIndex};
use futures::{channel::oneshot, Future, TryFutureExt};
use sc_client_api::{backend::Backend, Finalizer, LockImportRun, TransactionFor};
use sc_service::SpawnTaskHandle;
use sp_api::{NumberFor, ProvideRuntimeApi};
use sp_blockchain::{HeaderBackend, HeaderMetadata};
use sp_consensus::{BlockImport, SelectChain};
use sp_runtime::{
    traits::{BlakeTwo256, Block},
    RuntimeAppPublic,
};
use std::{convert::TryInto, fmt::Debug, sync::Arc};
mod aggregator;
pub mod config;
mod data_io;
mod finalization;
mod hash;
mod import;
mod justification;
pub mod metrics;
mod network;
mod party;

pub use import::AlephBlockImport;
pub use justification::JustificationNotification;

#[derive(Clone, Debug, Encode, Decode)]
enum Error {
    SendData,
}

pub fn peers_set_config() -> sc_network::config::NonDefaultSetConfig {
    sc_network::config::NonDefaultSetConfig {
        notifications_protocol: network::ALEPH_PROTOCOL_NAME.into(),
        // max_notification_size should be larger than the maximum possible honest message size (in bytes).
        // Max size of alert is UNIT_SIZE * MAX_UNITS_IN_ALERT ~ 100 * 5000 = 50000 bytes
        // Max size of parents response UNIT_SIZE * N_MEMBERS ~ 100 * N_MEMBERS
        // When adding other (large) message types we need to make sure this limit is fine.
        max_notification_size: 1024 * 1024,
        set_config: sc_network::config::SetConfig {
            // This seems to be a way to configure the AlephBFT network to have a prespecified set of nodes or at least
            // set suitable limits on the number of nodes we should connect to.
            in_peers: 0,
            out_peers: 0,
            reserved_nodes: vec![],
            non_reserved_mode: sc_network::config::NonReservedPeerMode::Accept,
        },
        fallback_names: Vec::new(),
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode)]
pub struct SessionId(pub u32);

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode)]
pub struct SessionPeriod(pub u32);

use sp_core::crypto::KeyTypeId;
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"alp0");
pub use crate::metrics::Metrics;
use crate::party::{run_consensus_party, AlephParams};
pub use aleph_primitives::{AuthorityId, AuthorityPair, AuthoritySignature};
use futures::channel::mpsc;

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
        .unwrap()
        .unwrap()
        .try_into()
        .unwrap()
    }
}

pub trait ClientForAleph<B, BE>:
    LockImportRun<B, BE>
    + Finalizer<B, BE>
    + ProvideRuntimeApi<B>
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
        + ProvideRuntimeApi<B>
        + HeaderBackend<B>
        + HeaderMetadata<B, Error = sp_blockchain::Error>
        + BlockImport<B, Transaction = TransactionFor<BE, B>, Error = sp_consensus::Error>,
{
}

type Hasher = hash::Wrapper<BlakeTwo256>;

#[derive(Clone, Debug, Decode, Encode)]
struct Signature {
    id: NodeIndex,
    sgn: AuthoritySignature,
}

#[derive(Clone)]
struct KeyBox {
    id: NodeIndex,
    auth_keystore: AuthorityKeystore,
    authorities: Vec<AuthorityId>,
}

impl aleph_bft::Index for KeyBox {
    fn index(&self) -> NodeIndex {
        self.id
    }
}

#[async_trait::async_trait]
impl aleph_bft::KeyBox for KeyBox {
    type Signature = Signature;

    fn node_count(&self) -> NodeCount {
        self.authorities.len().into()
    }

    async fn sign(&self, msg: &[u8]) -> Signature {
        Signature {
            id: self.id,
            sgn: self.auth_keystore.sign(msg),
        }
    }
    fn verify(&self, msg: &[u8], sgn: &Signature, index: NodeIndex) -> bool {
        self.authorities[index.0].verify(&msg.to_vec(), &sgn.sgn)
    }
}

type MultiKeychain = DefaultMultiKeychain<KeyBox>;

#[derive(Clone)]
struct SpawnHandle(SpawnTaskHandle);

impl From<SpawnTaskHandle> for SpawnHandle {
    fn from(sth: SpawnTaskHandle) -> Self {
        SpawnHandle(sth)
    }
}

impl aleph_bft::SpawnHandle for SpawnHandle {
    fn spawn(&self, name: &'static str, task: impl Future<Output = ()> + Send + 'static) {
        self.0.spawn(name, task)
    }
    fn spawn_essential(
        &self,
        name: &'static str,
        task: impl Future<Output = ()> + Send + 'static,
    ) -> TaskHandle {
        let (tx, rx) = oneshot::channel();
        self.0.spawn(name, async move {
            task.await;
            let _ = tx.send(());
        });
        Box::pin(rx.map_err(|_| ()))
    }
}

pub fn last_block_of_session<B: Block>(
    session_id: SessionId,
    period: SessionPeriod,
) -> NumberFor<B> {
    ((session_id.0 + 1) * period.0 - 1).into()
}

pub fn session_id_from_block_num<B: Block>(num: NumberFor<B>, period: SessionPeriod) -> SessionId
where
    NumberFor<B>: Into<u32>,
{
    SessionId(num.into() / period.0)
}

pub struct AlephConfig<B: Block, N, C, SC> {
    pub network: N,
    pub client: Arc<C>,
    pub select_chain: SC,
    pub spawn_handle: SpawnTaskHandle,
    pub auth_keystore: AuthorityKeystore,
    pub justification_rx: mpsc::UnboundedReceiver<JustificationNotification<B>>,
    pub metrics: Option<Metrics<B::Header>>,
    pub period: SessionPeriod,
}

pub fn run_aleph_consensus<B: Block, BE, C, N, SC>(
    config: AlephConfig<B, N, C, SC>,
) -> impl Future<Output = ()>
where
    BE: Backend<B> + 'static,
    N: network::Network<B> + 'static,
    C: ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B, AuthorityId, NumberFor<B>>,
    SC: SelectChain<B> + 'static,
    NumberFor<B>: Into<u32>,
{
    run_consensus_party(AlephParams { config })
}
