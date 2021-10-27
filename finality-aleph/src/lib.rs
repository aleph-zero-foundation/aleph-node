use codec::{Decode, Encode};

pub use aleph_bft::default_config as default_aleph_config;
use aleph_bft::{NodeIndex, TaskHandle};
use futures::{channel::oneshot, Future, TryFutureExt};
use sc_client_api::{backend::Backend, BlockchainEvents, Finalizer, LockImportRun, TransactionFor};
use sc_consensus::BlockImport;
use sc_service::SpawnTaskHandle;
use sp_api::{NumberFor, ProvideRuntimeApi};
use sp_blockchain::{HeaderBackend, HeaderMetadata};
use sp_consensus::SelectChain;
use sp_keystore::CryptoStore;
use sp_runtime::{
    traits::{BlakeTwo256, Block},
    SaturatedConversion,
};
use std::{collections::HashMap, fmt::Debug, sync::Arc};
mod aggregator;
pub mod config;
mod crypto;
mod data_io;
mod finalization;
mod hash;
mod import;
mod justification;
pub mod metrics;
mod network;
mod party;
#[cfg(test)]
pub mod testing;

pub use import::AlephBlockImport;
pub use justification::JustificationNotification;

#[derive(Clone, Debug, Encode, Decode)]
enum Error {
    SendData,
}

pub fn peers_set_config() -> sc_network::config::NonDefaultSetConfig {
    let mut config = sc_network::config::NonDefaultSetConfig::new(
        network::ALEPH_PROTOCOL_NAME.into(),
        // max_notification_size should be larger than the maximum possible honest message size (in bytes).
        // Max size of alert is UNIT_SIZE * MAX_UNITS_IN_ALERT ~ 100 * 5000 = 50000 bytes
        // Max size of parents response UNIT_SIZE * N_MEMBERS ~ 100 * N_MEMBERS
        // When adding other (large) message types we need to make sure this limit is fine.
        1024 * 1024,
    );
    config.set_config = sc_network::config::SetConfig::default();
    config
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd, Encode, Decode)]
pub struct SessionId(pub u32);

pub use crate::metrics::Metrics;
use crate::party::{run_consensus_party, AlephParams};
pub use aleph_primitives::{AuthorityId, AuthorityPair, AuthoritySignature};
use aleph_primitives::{MillisecsPerBlock, SessionPeriod, UnitCreationDelay};
use futures::channel::mpsc;

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
        self.spawn(name, async move {
            task.await;
            let _ = tx.send(());
        });
        Box::pin(rx.map_err(|_| ()))
    }
}

pub type SessionMap = HashMap<SessionId, Vec<AuthorityId>>;

pub fn last_block_of_session<B: Block>(
    session_id: SessionId,
    period: SessionPeriod,
) -> NumberFor<B> {
    ((session_id.0 + 1) * period.0 - 1).into()
}

pub fn session_id_from_block_num<B: Block>(num: NumberFor<B>, period: SessionPeriod) -> SessionId {
    SessionId(num.saturated_into::<u32>() / period.0)
}

pub struct AlephConfig<B: Block, N, C, SC> {
    pub network: N,
    pub client: Arc<C>,
    pub select_chain: SC,
    pub spawn_handle: SpawnTaskHandle,
    pub keystore: Arc<dyn CryptoStore>,
    pub justification_rx: mpsc::UnboundedReceiver<JustificationNotification<B>>,
    pub metrics: Option<Metrics<B::Header>>,
    pub session_period: SessionPeriod,
    pub millisecs_per_block: MillisecsPerBlock,
    pub unit_creation_delay: UnitCreationDelay,
}

pub fn run_aleph_consensus<B: Block, BE, C, N, SC>(
    config: AlephConfig<B, N, C, SC>,
) -> impl Future<Output = ()>
where
    BE: Backend<B> + 'static,
    N: network::Network<B> + network::RequestBlocks<B> + 'static,
    C: ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: aleph_primitives::AlephSessionApi<B>,
    SC: SelectChain<B> + 'static,
{
    run_consensus_party(AlephParams { config })
}
