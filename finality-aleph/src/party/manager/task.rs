use std::{boxed::Box, pin::Pin};

use futures::channel::oneshot;
use log::warn;

use crate::Future;

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
