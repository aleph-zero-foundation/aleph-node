use crate::{
    crypto::KeyBox,
    data_io::{AlephData, OrderedDataInterpreter},
    network::{AlephNetworkData, DataNetwork, NetworkWrapper},
    party::{AuthoritySubtaskCommon, Task},
};
use aleph_bft::{Config, LocalIO, SpawnHandle};
use futures::channel::oneshot;
use log::debug;
use sc_client_api::HeaderBackend;
use sp_runtime::traits::Block;
use std::io::{empty, sink};

/// Runs the member within a single session.
pub fn task<
    B: Block,
    C: HeaderBackend<B> + Send + 'static,
    ADN: DataNetwork<AlephNetworkData<B>> + 'static,
>(
    subtask_common: AuthoritySubtaskCommon,
    multikeychain: KeyBox,
    config: Config,
    network: NetworkWrapper<AlephNetworkData<B>, ADN>,
    data_provider: impl aleph_bft::DataProvider<AlephData<B>> + Send + 'static,
    ordered_data_interpreter: OrderedDataInterpreter<B, C>,
) -> Task {
    let AuthoritySubtaskCommon {
        spawn_handle,
        session_id,
    } = subtask_common;
    let (stop, exit) = oneshot::channel();
    // `sink` and `empty` here are noop placeholders which will be replaced in A0-542
    let local_io = LocalIO::new(data_provider, ordered_data_interpreter, sink(), empty());
    let task = {
        let spawn_handle = spawn_handle.clone();
        async move {
            debug!(target: "aleph-party", "Running the member task for {:?}", session_id);
            aleph_bft::run_session(config, local_io, network, multikeychain, spawn_handle, exit)
                .await;
            debug!(target: "aleph-party", "Member task stopped for {:?}", session_id);
        }
    };

    let handle = spawn_handle.spawn_essential("aleph/consensus_session_member", task);
    Task::new(handle, stop)
}
