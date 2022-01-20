use crate::Future;
use futures::channel::oneshot;
use log::warn;
use std::{boxed::Box, pin::Pin};

/// A single handle that can be waited on, as returned by spawning an essential task.
pub type Handle = Pin<Box<(dyn Future<Output = sc_service::Result<(), ()>> + Send + 'static)>>;

/// A task that can be stopped or awaited until it stops itself.
pub struct Task {
    handle: Handle,
    exit: oneshot::Sender<()>,
}

impl Task {
    /// Create a new task.
    pub fn new(handle: Handle, exit: oneshot::Sender<()>) -> Self {
        Task { handle, exit }
    }

    /// Cleanly stop the task.
    pub async fn stop(self) {
        if let Err(e) = self.exit.send(()) {
            warn!(target: "aleph-party", "Failed to send exit signal to authority: {:?}", e);
        } else {
            let _ = self.handle.await;
        }
    }

    /// Await the task to stop by itself. Should usually just block forever, unless something went
    /// wrong.
    pub async fn stopped(&mut self) {
        let _ = (&mut self.handle).await;
    }
}
