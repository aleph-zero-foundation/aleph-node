use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use futures::{
    channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender},
    StreamExt,
};
use tokio::time::timeout;

#[derive(Clone)]
pub(crate) struct SingleActionMock<CallArgs: Send> {
    timeout: Duration,
    history_tx: Arc<Mutex<UnboundedSender<CallArgs>>>,
    history_rx: Arc<Mutex<UnboundedReceiver<CallArgs>>>,
}

unsafe impl<CallArgs: Send> Send for SingleActionMock<CallArgs> {}

impl<CallArgs: Send> SingleActionMock<CallArgs> {
    pub(crate) fn new(timeout: Duration) -> Self {
        let (history_tx, history_rx) = unbounded();
        Self {
            timeout,
            history_tx: Arc::new(Mutex::new(history_tx)),
            history_rx: Arc::new(Mutex::new(history_rx)),
        }
    }

    pub(crate) fn invoke_with(&self, args: CallArgs) {
        self.history_tx
            .lock()
            .unwrap()
            .unbounded_send(args)
            .unwrap()
    }

    //This code is used only for testing.
    #[allow(clippy::await_holding_lock)]
    pub(crate) async fn has_not_been_invoked(&self) -> bool {
        timeout(self.timeout, self.history_rx.lock().unwrap().next())
            .await
            .is_err()
    }

    //This code is used only for testing.
    #[allow(clippy::await_holding_lock)]
    #[allow(clippy::significant_drop_in_scrutinee)]
    pub(crate) async fn has_been_invoked_with<P: FnOnce(CallArgs) -> bool>(
        &self,
        predicate: P,
    ) -> bool {
        match timeout(self.timeout, self.history_rx.lock().unwrap().next()).await {
            Ok(Some(args)) => predicate(args),
            _ => false,
        }
    }
}

const DEFAULT_TIMEOUT: Duration = Duration::from_millis(50);

impl<CallArgs: Send> Default for SingleActionMock<CallArgs> {
    fn default() -> Self {
        SingleActionMock::new(DEFAULT_TIMEOUT)
    }
}
