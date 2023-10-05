use std::collections::HashMap;

use substrate_prometheus_endpoint::{
    exponential_buckets, prometheus::HistogramTimer, register, CounterVec, Histogram,
    HistogramOpts, Opts, PrometheusError, Registry, U64,
};

use crate::Protocol;

fn protocol_name(protocol: Protocol) -> &'static str {
    use Protocol::*;
    match protocol {
        Authentication => "authentication",
        BlockSync => "block_sync",
    }
}

#[derive(Clone)]
pub enum Metrics {
    Prometheus {
        send_times: HashMap<Protocol, Histogram>,
        peer_sender_queue_size: CounterVec<U64>,
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
                            subsystem: protocol_name(protocol).to_string(),
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

        let peer_sender_queue_size = register(CounterVec::new(
            Opts::new(
                "gossip_network_peer_sender_queue",
                "Total number of messages sent and received by peer sender queues for all peers, for a given protocol",
            ),
            &["protocol", "action"],
        )?, &registry)?;

        Ok(Metrics::Prometheus {
            send_times,
            peer_sender_queue_size,
        })
    }

    pub fn noop() -> Self {
        Metrics::Noop
    }

    pub fn start_sending_in(&self, protocol: Protocol) -> Option<HistogramTimer> {
        match self {
            Metrics::Prometheus { send_times, .. } => send_times
                .get(&protocol)
                .map(|histogram| histogram.start_timer()),
            Metrics::Noop => None,
        }
    }

    pub fn report_message_pushed_to_peer_sender_queue(&self, protocol: Protocol) {
        match self {
            Metrics::Prometheus {
                peer_sender_queue_size,
                ..
            } => {
                peer_sender_queue_size
                    .with_label_values(&[protocol_name(protocol), "send"])
                    .inc();
            }
            Metrics::Noop => {}
        }
    }

    pub fn report_message_popped_from_peer_sender_queue(&self, protocol: Protocol) {
        match self {
            Metrics::Prometheus {
                peer_sender_queue_size,
                ..
            } => {
                peer_sender_queue_size
                    .with_label_values(&[protocol_name(protocol), "received"])
                    .inc();
            }
            Metrics::Noop => {}
        }
    }
}
