use std::collections::HashMap;

use substrate_prometheus_endpoint::{register, Counter, PrometheusError, Registry, U64};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Event {
    Broadcast,
    SendRequest,
    SendTo,
    HandleState,
    HandleRequestResponse,
    HandleRequest,
    HandleTask,
    HandleBlockImported,
    HandleBlockFinalized,
    HandleStateResponse,
    HandleJustificationFromUser,
    HandleInternalRequest,
}

use Event::*;

impl Event {
    fn name(&self) -> &str {
        match self {
            Broadcast => "broadcast",
            SendRequest => "send_request",
            SendTo => "send_to",
            HandleState => "handle_state",
            HandleRequestResponse => "handle_request_response",
            HandleRequest => "handle_request",
            HandleTask => "handle_task",
            HandleBlockImported => "handle_block_imported",
            HandleBlockFinalized => "handle_block_finalized",
            HandleStateResponse => "handle_state_response",
            HandleJustificationFromUser => "handle_justification_from_user",
            HandleInternalRequest => "handle_internal_request",
        }
    }
}

const ALL_EVENTS: [Event; 12] = [
    Broadcast,
    SendRequest,
    SendTo,
    HandleState,
    HandleRequestResponse,
    HandleRequest,
    HandleTask,
    HandleBlockImported,
    HandleBlockFinalized,
    HandleStateResponse,
    HandleJustificationFromUser,
    HandleInternalRequest,
];

const ERRORING_EVENTS: [Event; 9] = [
    Broadcast,
    SendRequest,
    SendTo,
    HandleState,
    HandleRequest,
    HandleTask,
    HandleBlockImported,
    HandleJustificationFromUser,
    HandleInternalRequest,
];

pub enum Metrics {
    Prometheus {
        calls: HashMap<Event, Counter<U64>>,
        errors: HashMap<Event, Counter<U64>>,
    },
    Noop,
}

impl Metrics {
    pub fn new(registry: Option<Registry>) -> Result<Self, PrometheusError> {
        let registry = match registry {
            Some(registry) => registry,
            None => return Ok(Metrics::Noop),
        };
        let mut calls = HashMap::new();
        let mut errors = HashMap::new();
        for event in ALL_EVENTS {
            calls.insert(
                event,
                register(
                    Counter::new(
                        format!("aleph_sync_{}", event.name()),
                        format!("number of times {} has been called", event.name()),
                    )?,
                    &registry,
                )?,
            );
        }
        for event in ERRORING_EVENTS {
            errors.insert(
                event,
                register(
                    Counter::new(
                        format!("aleph_sync_{}_error", event.name()),
                        format!("number of times {} has returned an error", event.name()),
                    )?,
                    &registry,
                )?,
            );
        }
        Ok(Metrics::Prometheus { calls, errors })
    }

    pub fn noop() -> Self {
        Metrics::Noop
    }

    pub fn report_event(&self, event: Event) {
        if let Metrics::Prometheus { calls, .. } = self {
            if let Some(counter) = calls.get(&event) {
                counter.inc();
            }
        }
    }

    pub fn report_event_error(&self, event: Event) {
        if let Metrics::Prometheus { errors, .. } = self {
            if let Some(counter) = errors.get(&event) {
                counter.inc();
            }
        }
    }
}
