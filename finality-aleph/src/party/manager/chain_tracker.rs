use aleph_primitives::BlockNumber;
use futures::channel::oneshot;
use log::debug;
use sc_client_api::HeaderBackend;
use sp_consensus::SelectChain;
use sp_runtime::traits::{Block, Header};

use crate::{
    abft::SpawnHandleT,
    data_io::ChainTracker,
    party::{AuthoritySubtaskCommon, Task},
};

/// Runs the latest block refresher within a single session.
pub fn task<B, SC, C>(
    subtask_common: AuthoritySubtaskCommon,
    chain_tracker: ChainTracker<B, SC, C>,
) -> Task
where
    B: Block,
    B::Header: Header<Number = BlockNumber>,
    C: HeaderBackend<B> + 'static,
    SC: SelectChain<B> + 'static,
{
    let AuthoritySubtaskCommon {
        spawn_handle,
        session_id,
    } = subtask_common;
    let (stop, exit) = oneshot::channel();
    let task = async move {
        debug!(target: "aleph-party", "Running the chain refresh task for {:?}", session_id);
        chain_tracker.run(exit).await;
        debug!(target: "aleph-party", "Chain refresh task stopped for {:?}", session_id);
    };

    let handle = spawn_handle.spawn_essential("aleph/consensus_session_refresher", task);
    Task::new(handle, stop)
}
