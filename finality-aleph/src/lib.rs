use std::{fmt::Debug, path::PathBuf, sync::Arc};

use aleph_bft::{NodeIndex, TaskHandle};
use codec::{Decode, Encode};
use futures::{
    channel::{mpsc, oneshot},
    Future, TryFutureExt,
};
use sc_client_api::{backend::Backend, BlockchainEvents, Finalizer, LockImportRun, TransactionFor};
use sc_consensus::BlockImport;
use sc_network::{ExHashT, NetworkService};
use sc_service::SpawnTaskHandle;
use sp_api::{NumberFor, ProvideRuntimeApi};
use sp_blockchain::{HeaderBackend, HeaderMetadata};
use sp_keystore::CryptoStore;
use sp_runtime::traits::{BlakeTwo256, Block, Header};

use crate::{
    aggregation::RmcNetworkData,
    network::{AlephNetworkData, Split},
    session::{
        first_block_of_session, last_block_of_session, session_id_from_block_num,
        SessionBoundaries, SessionId,
    },
    substrate_network::protocol_name,
};

mod aggregation;
mod crypto;
mod data_io;
mod finalization;
mod hash;
mod import;
mod justification;
pub mod metrics;
mod network;
mod nodes;
mod party;
mod session;
mod session_map;
mod substrate_network;
#[cfg(test)]
pub mod testing;

pub use aleph_bft::default_config as default_aleph_config;
pub use aleph_primitives::{AuthorityId, AuthorityPair, AuthoritySignature};
pub use import::AlephBlockImport;
pub use justification::{AlephJustification, JustificationNotification};
pub use network::Protocol;
pub use nodes::{run_nonvalidator_node, run_validator_node};
pub use session::SessionPeriod;

pub use crate::metrics::Metrics;

#[derive(Clone, Debug, Encode, Decode)]
enum Error {
    SendData,
}

/// Returns a NonDefaultSetConfig for the specified protocol.
pub fn peers_set_config(protocol: Protocol) -> sc_network::config::NonDefaultSetConfig {
    let name = protocol_name(&protocol);

    let mut config = sc_network::config::NonDefaultSetConfig::new(
        name,
        // max_notification_size should be larger than the maximum possible honest message size (in bytes).
        // Max size of alert is UNIT_SIZE * MAX_UNITS_IN_ALERT ~ 100 * 5000 = 50000 bytes
        // Max size of parents response UNIT_SIZE * N_MEMBERS ~ 100 * N_MEMBERS
        // When adding other (large) message types we need to make sure this limit is fine.
        1024 * 1024,
    );

    config.set_config = match protocol {
        // No spontaneous connections, only reserved nodes added by the network logic.
        Protocol::Validator => sc_network::config::SetConfig {
            in_peers: 0,
            out_peers: 0,
            reserved_nodes: Vec::new(),
            non_reserved_mode: sc_network::config::NonReservedPeerMode::Deny,
        },
        Protocol::Generic => sc_network::config::SetConfig::default(),
    };
    config
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode)]
pub struct MillisecsPerBlock(pub u64);

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode)]
pub struct UnitCreationDelay(pub u64);

pub(crate) type SplitData<B> = Split<AlephNetworkData<B>, RmcNetworkData<B>>;

pub trait ClientForAleph<B, BE>:
    LockImportRun<B, BE>
    + Finalizer<B, BE>
    + ProvideRuntimeApi<B>
    + BlockImport<B, Transaction = TransactionFor<BE, B>, Error = sp_consensus::Error>
    + HeaderBackend<B>
    + HeaderMetadata<B, Error = sp_blockchain::Error>
    + BlockchainEvents<B>
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
        + BlockchainEvents<B>
        + BlockImport<B, Transaction = TransactionFor<BE, B>, Error = sp_consensus::Error>,
{
}

type Hasher = hash::Wrapper<BlakeTwo256>;

/// A wrapper for spawning tasks in a way compatible with AlephBFT.
#[derive(Clone)]
pub struct SpawnHandle(SpawnTaskHandle);

impl From<SpawnTaskHandle> for SpawnHandle {
    fn from(sth: SpawnTaskHandle) -> Self {
        SpawnHandle(sth)
    }
}

impl aleph_bft::SpawnHandle for SpawnHandle {
    fn spawn(&self, name: &'static str, task: impl Future<Output = ()> + Send + 'static) {
        self.0.spawn(name, None, task)
    }

    fn spawn_essential(
        &self,
        name: &'static str,
        task: impl Future<Output = ()> + Send + 'static,
    ) -> TaskHandle {
        let (tx, rx) = oneshot::channel();
        self.spawn(name, async move {
            task.await;
            let _ = tx.send(());
        });
        Box::pin(rx.map_err(|_| ()))
    }
}

impl SpawnHandle {
    fn spawn_essential_with_result(
        &self,
        name: &'static str,
        task: impl Future<Output = Result<(), ()>> + Send + 'static,
    ) -> TaskHandle {
        let (tx, rx) = oneshot::channel();
        let wrapped_task = async move {
            let result = task.await;
            let _ = tx.send(result);
        };
        let result = <Self as aleph_bft::SpawnHandle>::spawn_essential(self, name, wrapped_task);
        let wrapped_result = async move {
            let main_result = result.await;
            if main_result.is_err() {
                return Err(());
            }
            let rx_result = rx.await;
            rx_result.unwrap_or(Err(()))
        };
        Box::pin(wrapped_result)
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct HashNum<H, N> {
    hash: H,
    num: N,
}

impl<H, N> HashNum<H, N> {
    fn new(hash: H, num: N) -> Self {
        HashNum { hash, num }
    }
}

impl<H, N> From<(H, N)> for HashNum<H, N> {
    fn from(pair: (H, N)) -> Self {
        HashNum::new(pair.0, pair.1)
    }
}

pub type BlockHashNum<B> = HashNum<<B as Block>::Hash, NumberFor<B>>;

pub struct AlephConfig<B: Block, H: ExHashT, C, SC> {
    pub network: Arc<NetworkService<B, H>>,
    pub client: Arc<C>,
    pub select_chain: SC,
    pub spawn_handle: SpawnTaskHandle,
    pub keystore: Arc<dyn CryptoStore>,
    pub justification_rx: mpsc::UnboundedReceiver<JustificationNotification<B>>,
    pub metrics: Option<Metrics<<B::Header as Header>::Hash>>,
    pub session_period: SessionPeriod,
    pub millisecs_per_block: MillisecsPerBlock,
    pub unit_creation_delay: UnitCreationDelay,
    pub backup_saving_path: Option<PathBuf>,
}
