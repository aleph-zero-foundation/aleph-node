use std::collections::HashMap;

use substrate_prometheus_endpoint::{
    exponential_buckets, prometheus::HistogramTimer, register, Histogram, HistogramOpts, Opts,
    PrometheusError, Registry,
};

use crate::Protocol;

fn protocol_name(protocol: Protocol) -> String {
    use Protocol::*;
    match protocol {
        Authentication => "authentication",
        BlockSync => "block_sync",
    }
    .to_string()
}

#[derive(Clone)]
pub enum Metrics {
    Prometheus {
        send_times: HashMap<Protocol, Histogram>,
    },
    Noop,
}

impl Metrics {
    pub fn new(registry: Option<Registry>) -> Result<Self, PrometheusError> {
        use Protocol::*;
        let registry = match registry {
            Some(registry) => registry,
            None => return Ok(Metrics::Noop),
        };

        let mut send_times = HashMap::new();
        for protocol in [Authentication, BlockSync] {
            send_times.insert(
                protocol,
                register(
                    Histogram::with_opts(HistogramOpts {
                        common_opts: Opts {
                            namespace: "gossip_network".to_string(),
                            subsystem: protocol_name(protocol),
                            name: "send_duration".to_string(),
                            help: "How long did it take for substrate to send a message."
                                .to_string(),
                            const_labels: Default::default(),
                            variable_labels: Default::default(),
                        },
                        buckets: exponential_buckets(0.001, 1.26, 30)?,
                    })?,
                    &registry,
                )?,
            );
        }
        Ok(Metrics::Prometheus { send_times })
    }

    pub fn noop() -> Self {
        Metrics::Noop
    }

    pub fn start_sending_in(&self, protocol: Protocol) -> Option<HistogramTimer> {
        match self {
            Metrics::Prometheus { send_times } => send_times
                .get(&protocol)
                .map(|histogram| histogram.start_timer()),
            Metrics::Noop => None,
        }
    }
}
