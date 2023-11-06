use std::{boxed::Box, pin::Pin};

use futures::channel::oneshot;
use log::{debug, warn};
use network_clique::SpawnHandleT;

use crate::{Future, SpawnHandle};

/// A single handle that can be waited on, as returned by spawning an essential task.
pub type Handle = Pin<Box<(dyn Future<Output = sc_service::Result<(), ()>> + Send + 'static)>>;

/// A task that can be stopped or awaited until it stops itself.
pub struct Task {
    handle: Handle,
    exit: oneshot::Sender<()>,
    cached_result: Option<Result<(), ()>>,
}

impl Task {
    /// Create a new task.
    pub fn new(handle: Handle, exit: oneshot::Sender<()>) -> Self {
        Task {
            handle,
            exit,
            cached_result: None,
        }
    }

    /// Cleanly stop the task.
    pub async fn stop(self) -> Result<(), ()> {
        if let Some(result) = self.cached_result {
            return result;
        }
        if self.exit.send(()).is_err() {
            warn!(target: "aleph-party", "Failed to send exit signal to authority");
        }
        self.handle.await
    }

    /// Await the task to stop by itself. Should usually just block forever, unless something went
    /// wrong. Can be called multiple times.
    pub async fn stopped(&mut self) -> Result<(), ()> {
        if let Some(result) = self.cached_result {
            return result;
        }
        let result = (&mut self.handle).await;
        self.cached_result = Some(result);
        result
    }
}

/// Common args for tasks.
#[derive(Clone)]
pub struct TaskCommon {
    pub spawn_handle: SpawnHandle,
    pub session_id: u32,
}

#[async_trait::async_trait]
pub trait Runnable: Send + 'static {
    async fn run(self, exit: oneshot::Receiver<()>);
}

/// Runs the given task within a single session.
pub fn task<R: Runnable>(subtask_common: TaskCommon, runnable: R, name: &'static str) -> Task {
    let TaskCommon {
        spawn_handle,
        session_id,
    } = subtask_common;
    let (stop, exit) = oneshot::channel();
    let task = {
        async move {
            debug!(target: "aleph-party", "Running the {} task for {:?}", name, session_id);
            runnable.run(exit).await;
            debug!(target: "aleph-party", "The {} task stopped for {:?}", name, session_id);
        }
    };

    let handle = spawn_handle.spawn_essential("aleph/consensus_session_task", task);
    Task::new(handle, stop)
}
