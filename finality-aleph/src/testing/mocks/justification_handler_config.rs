use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{
    justification::{JustificationHandlerConfig, JustificationRequestScheduler, SchedulerActions},
    testing::mocks::{single_action_mock::SingleActionMock, AcceptancePolicy},
};

#[derive(Clone)]
pub(crate) struct JustificationRequestSchedulerImpl {
    acceptance_policy: Arc<Mutex<AcceptancePolicy>>,
    fin_reporter: SingleActionMock<()>,
    req_reporter: SingleActionMock<()>,
}

impl JustificationRequestSchedulerImpl {
    pub(crate) fn new(acceptance_policy: AcceptancePolicy) -> Self {
        Self {
            acceptance_policy: Arc::new(Mutex::new(acceptance_policy)),
            fin_reporter: Default::default(),
            req_reporter: Default::default(),
        }
    }

    pub(crate) fn update_policy(&self, policy: AcceptancePolicy) {
        *self.acceptance_policy.lock().unwrap() = policy;
    }

    pub(crate) async fn has_been_finalized(&self) -> bool {
        self.fin_reporter.has_been_invoked_with(|_| true).await
    }

    pub(crate) async fn has_been_requested(&self) -> bool {
        self.req_reporter.has_been_invoked_with(|_| true).await
    }
}

impl JustificationRequestScheduler for JustificationRequestSchedulerImpl {
    fn schedule_action(&mut self) -> SchedulerActions {
        if self.acceptance_policy.lock().unwrap().accepts() {
            SchedulerActions::Request
        } else {
            SchedulerActions::Wait
        }
    }

    fn on_block_finalized(&mut self) {
        self.fin_reporter.invoke_with(());
    }

    fn on_request_sent(&mut self) {
        self.req_reporter.invoke_with(());
    }
}

const DEFAULT_VERIFIER_TIMEOUT_MS: u64 = 10u64;
const DEFAULT_NOTIFICATION_TIMEOUT_MS: u64 = 10u64;

impl JustificationHandlerConfig {
    pub fn test() -> Self {
        JustificationHandlerConfig::new(
            Duration::from_millis(DEFAULT_VERIFIER_TIMEOUT_MS),
            Duration::from_millis(DEFAULT_NOTIFICATION_TIMEOUT_MS),
        )
    }
}
