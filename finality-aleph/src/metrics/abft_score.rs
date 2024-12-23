use substrate_prometheus_endpoint::{register, Gauge, PrometheusError, Registry, U64};

#[derive(Clone)]
pub enum ScoreMetrics {
    Prometheus { my_score: Gauge<U64> },
    Noop,
}

impl ScoreMetrics {
    pub fn new(registry: Option<Registry>) -> Result<Self, PrometheusError> {
        match registry {
            Some(registry) => Ok(ScoreMetrics::Prometheus {
                my_score: register(
                    Gauge::new("my_abft_score", "My abft score observed in last batch")?,
                    &registry,
                )?,
            }),
            None => Ok(ScoreMetrics::Noop),
        }
    }

    pub fn noop() -> Self {
        ScoreMetrics::Noop
    }

    pub fn report_score(&self, score: u16) {
        if let ScoreMetrics::Prometheus { my_score } = self {
            my_score.set(score.into());
        }
    }
}
