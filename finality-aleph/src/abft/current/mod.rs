use std::time::Duration;

use current_aleph_bft::{create_config, default_delay_config, Config, LocalIO, Terminator};
use log::debug;
use network_clique::SpawnHandleT;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::{Block, Header};

mod network;
mod traits;

pub use network::NetworkData;

pub use crate::aleph_primitives::{BlockHash, BlockNumber, CURRENT_FINALITY_VERSION as VERSION};
use crate::{
    abft::{
        common::{unit_creation_delay_fn, MAX_ROUNDS, SESSION_LEN_LOWER_BOUND_MS},
        NetworkWrapper,
    },
    block::{Header as BlockHeader, HeaderVerifier, UnverifiedHeader},
    crypto::Signature,
    data_io::{AlephData, OrderedDataInterpreter, SubstrateChainInfoProvider},
    network::data::Network,
    oneshot,
    party::{
        backup::ABFTBackup,
        manager::{Task, TaskCommon},
    },
    CurrentNetworkData, Hasher, Keychain, NodeIndex, SessionId, SignatureSet, UnitCreationDelay,
};

type WrappedNetwork<H, ADN> = NetworkWrapper<
    current_aleph_bft::NetworkData<Hasher, AlephData<H>, Signature, SignatureSet<Signature>>,
    ADN,
>;

pub fn run_member<B, C, ADN, V>(
    subtask_common: TaskCommon,
    multikeychain: Keychain,
    config: Config,
    network: WrappedNetwork<B::Header, ADN>,
    data_provider: impl current_aleph_bft::DataProvider<AlephData<B::Header>> + Send + 'static,
    ordered_data_interpreter: OrderedDataInterpreter<
        SubstrateChainInfoProvider<B, C>,
        B::Header,
        V,
    >,
    backup: ABFTBackup,
) -> Task
where
    B: Block<Hash = BlockHash>,
    B::Header:
        Header<Number = BlockNumber> + UnverifiedHeader + BlockHeader<Unverified = B::Header>,
    C: HeaderBackend<B> + Send + 'static,
    ADN: Network<CurrentNetworkData<B::Header>> + 'static,
    V: HeaderVerifier<B::Header>,
{
    let TaskCommon {
        spawn_handle,
        session_id,
    } = subtask_common;
    let (stop, exit) = oneshot::channel();
    let member_terminator = Terminator::create_root(exit, "member");
    let local_io = LocalIO::new(data_provider, ordered_data_interpreter, backup.0, backup.1);

    let task = {
        let spawn_handle = spawn_handle.clone();
        async move {
            debug!(target: "aleph-party", "Running the member task for {:?}", session_id);
            current_aleph_bft::run_session(
                config,
                local_io,
                network,
                multikeychain,
                spawn_handle,
                member_terminator,
            )
            .await;
            debug!(target: "aleph-party", "Member task stopped for {:?}", session_id);
        }
    };

    let handle = spawn_handle.spawn_essential("aleph/consensus_session_member", task);
    Task::new(handle, stop)
}

pub fn create_aleph_config(
    n_members: usize,
    node_id: NodeIndex,
    session_id: SessionId,
    unit_creation_delay: UnitCreationDelay,
) -> Config {
    let mut delay_config = default_delay_config();
    delay_config.unit_creation_delay = unit_creation_delay_fn(unit_creation_delay);
    match create_config(n_members.into(), node_id.into(), session_id.0 as u64, MAX_ROUNDS, delay_config, Duration::from_millis(SESSION_LEN_LOWER_BOUND_MS as u64)) {
        Ok(config) => config,
        Err(_) => panic!("Incorrect setting of delays. Make sure the total AlephBFT session time is at least {} ms.", SESSION_LEN_LOWER_BOUND_MS),
    }
}
