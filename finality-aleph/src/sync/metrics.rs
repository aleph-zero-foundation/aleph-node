use std::collections::HashMap;

use substrate_prometheus_endpoint::{register, Counter, Gauge, PrometheusError, Registry, U64};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Event {
    Broadcast,
    SendRequest,
    SendTo,
    SendExtensionRequest,
    HandleState,
    HandleRequestResponse,
    HandleRequest,
    HandleExtensionRequest,
    HandleTask,
    HandleBlockImported,
    HandleBlockFinalized,
    HandleStateResponse,
    HandleJustificationFromUser,
    HandleInternalRequest,
}

use Event::*;

use crate::BlockNumber;

impl Event {
    fn name(&self) -> &str {
        match self {
            Broadcast => "broadcast",
            SendRequest => "send_request",
            SendTo => "send_to",
            SendExtensionRequest => "send_extension_request",
            HandleState => "handle_state",
            HandleRequestResponse => "handle_request_response",
            HandleRequest => "handle_request",
            HandleExtensionRequest => "handle_extension_request",
            HandleTask => "handle_task",
            HandleBlockImported => "handle_block_imported",
            HandleBlockFinalized => "handle_block_finalized",
            HandleStateResponse => "handle_state_response",
            HandleJustificationFromUser => "handle_justification_from_user",
            HandleInternalRequest => "handle_internal_request",
        }
    }
}

const ALL_EVENTS: [Event; 14] = [
    Broadcast,
    SendRequest,
    SendTo,
    SendExtensionRequest,
    HandleState,
    HandleRequestResponse,
    HandleRequest,
    HandleExtensionRequest,
    HandleTask,
    HandleBlockImported,
    HandleBlockFinalized,
    HandleStateResponse,
    HandleJustificationFromUser,
    HandleInternalRequest,
];

const ERRORING_EVENTS: [Event; 11] = [
    Broadcast,
    SendRequest,
    SendTo,
    SendExtensionRequest,
    HandleState,
    HandleRequest,
    HandleExtensionRequest,
    HandleTask,
    HandleBlockImported,
    HandleJustificationFromUser,
    HandleInternalRequest,
];

pub enum Metrics {
    Prometheus {
        event_calls: HashMap<Event, Counter<U64>>,
        event_errors: HashMap<Event, Counter<U64>>,
        top_finalized_block: Gauge<U64>,
        best_block: Gauge<U64>,
    },
    Noop,
}

impl Metrics {
    pub fn new(registry: Option<Registry>) -> Result<Self, PrometheusError> {
        let registry = match registry {
            Some(registry) => registry,
            None => return Ok(Metrics::Noop),
        };
        let mut event_calls = HashMap::new();
        let mut event_errors = HashMap::new();
        for event in ALL_EVENTS {
            event_calls.insert(
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
            event_errors.insert(
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
        Ok(Metrics::Prometheus {
            event_calls,
            event_errors,
            top_finalized_block: register(
                Gauge::new("aleph_top_finalized_block", "no help")?,
                &registry,
            )?,
            best_block: register(Gauge::new("aleph_best_block", "no help")?, &registry)?,
        })
    }

    pub fn noop() -> Self {
        Metrics::Noop
    }

    pub fn report_event(&self, event: Event) {
        if let Metrics::Prometheus { event_calls, .. } = self {
            if let Some(counter) = event_calls.get(&event) {
                counter.inc();
            }
        }
    }

    pub fn report_event_error(&self, event: Event) {
        if let Metrics::Prometheus { event_errors, .. } = self {
            if let Some(counter) = event_errors.get(&event) {
                counter.inc();
            }
        }
    }

    pub fn update_best_block(&self, number: BlockNumber) {
        if let Metrics::Prometheus { best_block, .. } = self {
            best_block.set(number as u64)
        }
    }

    pub fn update_top_finalized_block(&self, number: BlockNumber) {
        if let Metrics::Prometheus {
            top_finalized_block,
            ..
        } = self
        {
            top_finalized_block.set(number as u64);
        }
    }
}
