pub trait NetworkCliqueMetrics {
    fn set_incoming_connections(&self, present: u64);
    fn set_missing_incoming_connections(&self, missing: u64);
    fn set_outgoing_connections(&self, present: u64);
    fn set_missing_outgoing_connections(&self, missing: u64);
}

impl<M: NetworkCliqueMetrics> NetworkCliqueMetrics for Option<M> {
    fn set_incoming_connections(&self, present: u64) {
        if let Some(m) = self {
            m.set_incoming_connections(present)
        }
    }

    fn set_missing_incoming_connections(&self, missing: u64) {
        if let Some(m) = self {
            m.set_missing_incoming_connections(missing)
        }
    }

    fn set_outgoing_connections(&self, present: u64) {
        if let Some(m) = self {
            m.set_outgoing_connections(present)
        }
    }

    fn set_missing_outgoing_connections(&self, missing: u64) {
        if let Some(m) = self {
            m.set_missing_outgoing_connections(missing)
        }
    }
}

pub struct NoopMetrics;

impl NetworkCliqueMetrics for NoopMetrics {
    fn set_incoming_connections(&self, _: u64) {}
    fn set_missing_incoming_connections(&self, _: u64) {}
    fn set_outgoing_connections(&self, _: u64) {}
    fn set_missing_outgoing_connections(&self, _: u64) {}
}
