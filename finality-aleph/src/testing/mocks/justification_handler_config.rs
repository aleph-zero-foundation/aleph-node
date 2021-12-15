use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::justification::{JustificationHandlerConfig, JustificationRequestDelay};
use crate::testing::mocks::single_action_mock::SingleActionMock;
use crate::testing::mocks::{AcceptancePolicy, TBlock};

#[derive(Clone)]
pub(crate) struct JustificationRequestDelayImpl {
    acceptance_policy: Arc<Mutex<AcceptancePolicy>>,
    fin_reporter: SingleActionMock<()>,
    req_reporter: SingleActionMock<()>,
}

impl JustificationRequestDelayImpl {
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

impl JustificationRequestDelay for JustificationRequestDelayImpl {
    fn can_request_now(&self) -> bool {
        self.acceptance_policy.lock().unwrap().accepts()
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

impl JustificationHandlerConfig<TBlock, JustificationRequestDelayImpl> {
    pub(crate) fn new(request_policy: AcceptancePolicy) -> Self {
        JustificationHandlerConfig {
            justification_request_delay: JustificationRequestDelayImpl::new(request_policy),
            metrics: None,
            verifier_timeout: Duration::from_millis(DEFAULT_VERIFIER_TIMEOUT_MS),
            notification_timeout: Duration::from_millis(DEFAULT_NOTIFICATION_TIMEOUT_MS),
        }
    }
}

impl Clone for JustificationHandlerConfig<TBlock, JustificationRequestDelayImpl> {
    fn clone(&self) -> Self {
        Self {
            justification_request_delay: self.justification_request_delay.clone(),
            metrics: self.metrics.clone(),
            verifier_timeout: self.verifier_timeout,
            notification_timeout: self.notification_timeout,
        }
    }
}
