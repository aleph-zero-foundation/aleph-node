use crate::{
    party::{Handle, Task as PureTask},
    NodeIndex, SpawnHandle,
};
use futures::channel::oneshot;

/// A wrapper for running the authority task within a specific session.
pub struct Task {
    task: PureTask,
    node_id: NodeIndex,
}

impl Task {
    /// Create a new authority task. The handle should be the handle to the actual task.
    pub fn new(handle: Handle, node_id: NodeIndex, exit: oneshot::Sender<()>) -> Self {
        Task {
            task: PureTask::new(handle, exit),
            node_id,
        }
    }

    /// Stop the authority task and wait for it to finish.
    pub async fn stop(self) {
        self.task.stop().await
    }

    /// If the authority task stops for any reason, this returns the associated NodeIndex, which
    /// can be used to restart the task.
    pub async fn stopped(&mut self) -> NodeIndex {
        self.task.stopped().await;
        self.node_id
    }
}

/// All the subtasks required to participate in a session as an authority.
pub struct Subtasks {
    exit: oneshot::Receiver<()>,
    member: PureTask,
    aggregator: PureTask,
    forwarder: PureTask,
    refresher: PureTask,
    data_store: PureTask,
}

impl Subtasks {
    /// Create the subtask collection by passing in all the tasks.
    pub fn new(
        exit: oneshot::Receiver<()>,
        member: PureTask,
        aggregator: PureTask,
        forwarder: PureTask,
        refresher: PureTask,
        data_store: PureTask,
    ) -> Self {
        Subtasks {
            exit,
            member,
            aggregator,
            forwarder,
            refresher,
            data_store,
        }
    }

    async fn stop(self) {
        // both member and aggregator are implicitly using forwarder,
        // so we should force them to exit first to avoid any panics, i.e. `send on closed channel`
        self.member.stop().await;
        self.aggregator.stop().await;
        self.forwarder.stop().await;
        self.refresher.stop().await;
        self.data_store.stop().await;
    }

    /// Blocks until the task is done and returns true if it quit unexpectedly.
    pub async fn failed(mut self) -> bool {
        let result = tokio::select! {
            _ = &mut self.exit => false,
            _ = self.member.stopped() => true,
            _ = self.aggregator.stopped() => true,
            _ = self.forwarder.stopped() => true,
            _ = self.refresher.stopped() => true,
            _ = self.data_store.stopped() => true,
        };
        self.stop().await;
        result
    }
}

/// Common args for authority subtasks.
#[derive(Clone)]
pub struct SubtaskCommon {
    pub spawn_handle: SpawnHandle,
    pub session_id: u32,
}
