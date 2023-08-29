use legacy_aleph_bft::{default_config, Config, LocalIO, Terminator};
use log::debug;
use network_clique::SpawnHandleT;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::{Block, Header};

use super::common::{sanity_check_round_delays, unit_creation_delay_fn, MAX_ROUNDS};
pub use crate::aleph_primitives::{BlockHash, BlockNumber, LEGACY_FINALITY_VERSION as VERSION};
use crate::{
    abft::NetworkWrapper,
    data_io::{AlephData, OrderedDataInterpreter, SubstrateChainInfoProvider},
    network::data::Network,
    oneshot,
    party::{
        backup::ABFTBackup,
        manager::{SubtaskCommon, Task},
    },
    Keychain, LegacyNetworkData, NodeIndex, SessionId, UnitCreationDelay,
};

pub fn run_member<B, C, ADN>(
    subtask_common: SubtaskCommon,
    multikeychain: Keychain,
    config: Config,
    network: NetworkWrapper<LegacyNetworkData, ADN>,
    data_provider: impl legacy_aleph_bft::DataProvider<AlephData> + Send + 'static,
    ordered_data_interpreter: OrderedDataInterpreter<SubstrateChainInfoProvider<B, C>>,
    backup: ABFTBackup,
) -> Task
where
    B: Block<Hash = BlockHash>,
    B::Header: Header<Number = BlockNumber>,
    C: HeaderBackend<B> + Send + 'static,
    ADN: Network<LegacyNetworkData> + 'static,
{
    // Remove this check once we implement one on the AlephBFT side (A0-2583).
    // Checks that the total time of a session is at least 7 days.
    sanity_check_round_delays(
        config.max_round,
        config.delay_config.unit_creation_delay.clone(),
    );
    let SubtaskCommon {
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
            legacy_aleph_bft::run_session(
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
    let mut config = default_config(n_members.into(), node_id.into(), session_id.0 as u64);
    config.delay_config.unit_creation_delay = unit_creation_delay_fn(unit_creation_delay);
    config.max_round = MAX_ROUNDS;
    config
}
