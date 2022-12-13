use std::time::Duration;

use legacy_aleph_bft::{Config, LocalIO};
use log::debug;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block;

use crate::{
    abft::{
        common::{unit_creation_delay_fn, AlephConfig, DelayConfig},
        NetworkWrapper, SpawnHandleT,
    },
    data_io::{AlephData, OrderedDataInterpreter},
    network::DataNetwork,
    oneshot,
    party::{
        backup::ABFTBackup,
        manager::{SubtaskCommon, Task},
    },
    Keychain, LegacyNetworkData, NodeIndex, SessionId, UnitCreationDelay,
};

/// Version of the legacy abft
pub const VERSION: u32 = 0;

pub fn run_member<
    B: Block,
    C: HeaderBackend<B> + Send + 'static,
    ADN: DataNetwork<LegacyNetworkData<B>> + 'static,
>(
    subtask_common: SubtaskCommon,
    multikeychain: Keychain,
    config: Config,
    network: NetworkWrapper<LegacyNetworkData<B>, ADN>,
    data_provider: impl legacy_aleph_bft::DataProvider<AlephData<B>> + Send + 'static,
    ordered_data_interpreter: OrderedDataInterpreter<B, C>,
    backup: ABFTBackup,
) -> Task {
    let SubtaskCommon {
        spawn_handle,
        session_id,
    } = subtask_common;
    let (stop, exit) = oneshot::channel();
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
                exit,
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
    let delay_config = DelayConfig {
        tick_interval: Duration::from_millis(100),
        requests_interval: Duration::from_millis(3000),
        unit_rebroadcast_interval_min: Duration::from_millis(15000),
        unit_rebroadcast_interval_max: Duration::from_millis(20000),
        unit_creation_delay: unit_creation_delay_fn(unit_creation_delay),
    };

    AlephConfig::new(delay_config, n_members, node_id, session_id).into()
}
