use std::{
    fmt::{Debug, Display, Error as FmtError, Formatter},
    hash::Hash,
    path::PathBuf,
    sync::Arc,
};

use derive_more::Display;
use futures::{
    channel::{mpsc, oneshot},
    Future,
};
use parity_scale_codec::{Decode, Encode, Output};
use primitives as aleph_primitives;
use primitives::{AuthorityId, Block as AlephBlock, BlockHash, BlockNumber, Hash as AlephHash};
use sc_client_api::{
    Backend, BlockBackend, BlockchainEvents, Finalizer, LockImportRun, TransactionFor,
};
use sc_consensus::BlockImport;
use sc_network::NetworkService;
use sc_network_sync::SyncingService;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::{HeaderBackend, HeaderMetadata};
use sp_keystore::Keystore;
use sp_runtime::traits::{BlakeTwo256, Block};
use substrate_prometheus_endpoint::Registry;
use tokio::time::Duration;

use crate::{
    abft::{
        CurrentNetworkData, Keychain, LegacyNetworkData, NodeCount, NodeIndex, Recipient,
        SignatureSet, SpawnHandle, CURRENT_VERSION, LEGACY_VERSION,
    },
    aggregation::{CurrentRmcNetworkData, LegacyRmcNetworkData},
    compatibility::{Version, Versioned},
    network::data::split::Split,
    session::{SessionBoundaries, SessionBoundaryInfo, SessionId},
    VersionedTryFromError::{ExpectedNewGotOld, ExpectedOldGotNew},
};

mod abft;
mod aggregation;
mod compatibility;
mod crypto;
mod data_io;
mod finalization;
mod import;
mod justification;
mod metrics;
mod network;
mod nodes;
mod party;
mod session;
mod session_map;
mod sync;
mod sync_oracle;
#[cfg(test)]
pub mod testing;

pub use crate::{
    import::{AlephBlockImport, TracingBlockImport},
    justification::AlephJustification,
    metrics::TimingBlockMetrics,
    network::{Protocol, ProtocolNaming},
    nodes::run_validator_node,
    session::SessionPeriod,
    sync::{
        substrate::{BlockImporter, Justification},
        JustificationTranslator, SubstrateChainStatus,
    },
    sync_oracle::SyncOracle,
};

/// Constant defining how often components of finality-aleph should report their state
const STATUS_REPORT_INTERVAL: Duration = Duration::from_secs(20);

/// Returns a NonDefaultSetConfig for the specified protocol.
pub fn peers_set_config(
    naming: ProtocolNaming,
    protocol: Protocol,
) -> sc_network::config::NonDefaultSetConfig {
    let mut config = sc_network::config::NonDefaultSetConfig::new(
        naming.protocol_name(&protocol),
        // max_notification_size should be larger than the maximum possible honest message size (in bytes).
        // Max size of alert is UNIT_SIZE * MAX_UNITS_IN_ALERT ~ 100 * 5000 = 50000 bytes
        // Max size of parents response UNIT_SIZE * N_MEMBERS ~ 100 * N_MEMBERS
        // When adding other (large) message types we need to make sure this limit is fine.
        1024 * 1024,
    );

    config.set_config = sc_network::config::SetConfig::default();
    config.add_fallback_names(naming.fallback_protocol_names(&protocol));
    config
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode)]
pub struct MillisecsPerBlock(pub u64);

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode)]
pub struct UnitCreationDelay(pub u64);

type LegacySplitData = Split<LegacyNetworkData, LegacyRmcNetworkData>;
type CurrentSplitData = Split<CurrentNetworkData, CurrentRmcNetworkData>;

impl Versioned for LegacyNetworkData {
    const VERSION: Version = Version(LEGACY_VERSION);
}

impl Versioned for CurrentNetworkData {
    const VERSION: Version = Version(CURRENT_VERSION);
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
    fn decode<I: parity_scale_codec::Input>(
        input: &mut I,
    ) -> Result<Self, parity_scale_codec::Error> {
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

type VersionedNetworkData = VersionedEitherMessage<LegacySplitData, CurrentSplitData>;

#[derive(Debug, Display, Clone)]
pub enum VersionedTryFromError {
    ExpectedNewGotOld,
    ExpectedOldGotNew,
}

impl TryFrom<VersionedNetworkData> for LegacySplitData {
    type Error = VersionedTryFromError;

    fn try_from(value: VersionedNetworkData) -> Result<Self, Self::Error> {
        Ok(match value {
            VersionedEitherMessage::Left(data) => data,
            VersionedEitherMessage::Right(_) => return Err(ExpectedOldGotNew),
        })
    }
}
impl TryFrom<VersionedNetworkData> for CurrentSplitData {
    type Error = VersionedTryFromError;

    fn try_from(value: VersionedNetworkData) -> Result<Self, Self::Error> {
        Ok(match value {
            VersionedEitherMessage::Left(_) => return Err(ExpectedNewGotOld),
            VersionedEitherMessage::Right(data) => data,
        })
    }
}

impl From<LegacySplitData> for VersionedNetworkData {
    fn from(data: LegacySplitData) -> Self {
        VersionedEitherMessage::Left(data)
    }
}

impl From<CurrentSplitData> for VersionedNetworkData {
    fn from(data: CurrentSplitData) -> Self {
        VersionedEitherMessage::Right(data)
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
    + BlockBackend<B>
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
        + BlockImport<B, Transaction = TransactionFor<BE, B>, Error = sp_consensus::Error>
        + BlockBackend<B>,
{
}

type Hasher = abft::HashWrapper<BlakeTwo256>;

/// The identifier of a block, the least amount of knowledge we can have about a block.
#[derive(PartialEq, Eq, Clone, Debug, Encode, Decode, Hash)]
pub struct BlockId {
    hash: BlockHash,
    number: BlockNumber,
}

impl BlockId {
    pub fn new(hash: BlockHash, number: BlockNumber) -> Self {
        BlockId { hash, number }
    }

    pub fn number(&self) -> BlockNumber {
        self.number
    }
}

impl From<(BlockHash, BlockNumber)> for BlockId {
    fn from(pair: (BlockHash, BlockNumber)) -> Self {
        BlockId::new(pair.0, pair.1)
    }
}

impl Display for BlockId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(f, "#{} ({})", self.number, self.hash,)
    }
}

#[derive(Clone)]
pub struct RateLimiterConfig {
    /// Maximum bit-rate per node in bytes per second of the alephbft validator network.
    pub alephbft_bit_rate_per_connection: usize,
}

pub struct AlephConfig<C, SC> {
    pub network: Arc<NetworkService<AlephBlock, AlephHash>>,
    pub sync_network: Arc<SyncingService<AlephBlock>>,
    pub client: Arc<C>,
    pub chain_status: SubstrateChainStatus,
    pub import_queue_handle: BlockImporter,
    pub select_chain: SC,
    pub spawn_handle: SpawnHandle,
    pub keystore: Arc<dyn Keystore>,
    pub justification_rx: mpsc::UnboundedReceiver<Justification>,
    pub metrics: TimingBlockMetrics,
    pub registry: Option<Registry>,
    pub session_period: SessionPeriod,
    pub millisecs_per_block: MillisecsPerBlock,
    pub unit_creation_delay: UnitCreationDelay,
    pub backup_saving_path: Option<PathBuf>,
    pub external_addresses: Vec<String>,
    pub validator_port: u16,
    pub protocol_naming: ProtocolNaming,
    pub rate_limiter_config: RateLimiterConfig,
    pub sync_oracle: SyncOracle,
}
