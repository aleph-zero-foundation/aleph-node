use std::{convert::Infallible, fmt::Debug, path::PathBuf, sync::Arc};

use aleph_bft::{NodeIndex, TaskHandle};
use codec::{Decode, Encode, Output};
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
use tokio::time::Duration;

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
mod validator_network;

pub use aleph_bft::default_config as default_aleph_config;
pub use aleph_primitives::{AuthorityId, AuthorityPair, AuthoritySignature};
pub use import::AlephBlockImport;
pub use justification::{AlephJustification, JustificationNotification};
pub use network::Protocol;
pub use nodes::{run_nonvalidator_node, run_validator_node};
pub use session::SessionPeriod;

pub use crate::metrics::Metrics;

/// Constant defining how often components of finality-aleph should report their state
const STATUS_REPORT_INTERVAL: Duration = Duration::from_secs(20);

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

pub type SplitData<B> = Split<AlephNetworkData<B>, RmcNetworkData<B>>;

impl<B: Block> Versioned for AlephNetworkData<B> {
    const VERSION: Version = Version(0);
}

#[derive(Encode, Eq, Decode, PartialEq)]
pub struct Version(u32);

pub trait Versioned {
    const VERSION: Version;
}

/// The main purpose of this data type is to enable a seamless transition between protocol versions at the Network level. It
/// provides a generic implementation of the Decode and Encode traits (LE byte representation) by prepending byte
/// representations for provided type parameters with their version (they need to implement the `Versioned` trait). If one
/// provides data types that declares equal versions, the first data type parameter will have priority while decoding. Keep in
/// mind that in such case, `decode` might fail even if the second data type would be able decode provided byte representation.
#[derive(Clone)]
pub enum VersionedEitherMessage<L, R> {
    Left(L),
    Right(R),
}

impl<L: Versioned + Decode, R: Versioned + Decode> Decode for VersionedEitherMessage<L, R> {
    fn decode<I: codec::Input>(input: &mut I) -> Result<Self, codec::Error> {
        let version = Version::decode(input)?;
        if version == L::VERSION {
            return Ok(VersionedEitherMessage::Left(L::decode(input)?));
        }
        if version == R::VERSION {
            return Ok(VersionedEitherMessage::Right(R::decode(input)?));
        }
        Err("Invalid version while decoding VersionedEitherMessage".into())
    }
}

impl<L: Versioned + Encode, R: Versioned + Encode> Encode for VersionedEitherMessage<L, R> {
    fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
        match self {
            VersionedEitherMessage::Left(left) => {
                L::VERSION.encode_to(dest);
                left.encode_to(dest);
            }
            VersionedEitherMessage::Right(right) => {
                R::VERSION.encode_to(dest);
                right.encode_to(dest);
            }
        }
    }

    fn size_hint(&self) -> usize {
        match self {
            VersionedEitherMessage::Left(left) => L::VERSION.size_hint() + left.size_hint(),
            VersionedEitherMessage::Right(right) => R::VERSION.size_hint() + right.size_hint(),
        }
    }
}

pub type VersionedNetworkData<B> = VersionedEitherMessage<SplitData<B>, SplitData<B>>;

impl<B: Block> TryFrom<VersionedNetworkData<B>> for SplitData<B> {
    type Error = Infallible;

    fn try_from(value: VersionedNetworkData<B>) -> Result<Self, Self::Error> {
        Ok(match value {
            VersionedEitherMessage::Left(data) => data,
            VersionedEitherMessage::Right(data) => data,
        })
    }
}

impl<B: Block> From<SplitData<B>> for VersionedNetworkData<B> {
    fn from(data: SplitData<B>) -> Self {
        VersionedEitherMessage::Left(data)
    }
}

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
