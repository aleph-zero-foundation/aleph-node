use crate::{
    crypto::KeyBox,
    data_io::{DataProvider, FinalizationHandler},
    network::AlephNetwork,
    party::{AuthoritySubtaskCommon, Task},
};
use aleph_bft::{Config, SpawnHandle};
use futures::channel::oneshot;
use log::debug;
use sp_runtime::traits::Block;

/// Runs the member within a single session.
pub fn task<B: Block>(
    subtask_common: AuthoritySubtaskCommon,
    multikeychain: KeyBox,
    config: Config,
    network: AlephNetwork<B>,
    data_provider: DataProvider<B>,
    finalization_handler: FinalizationHandler<B>,
) -> Task {
    let AuthoritySubtaskCommon {
        spawn_handle,
        session_id,
    } = subtask_common;
    let (stop, exit) = oneshot::channel();
    let task = {
        let spawn_handle = spawn_handle.clone();
        async move {
            debug!(target: "aleph-party", "Running the member task for {:?}", session_id);
            aleph_bft::run_session(
                config,
                network,
                data_provider,
                finalization_handler,
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
