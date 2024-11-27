use std::time::Instant;

use futures::future::pending;

use crate::{token_bucket::SharedTokenBucket, RatePerSecond};

pub type SharedRateLimiter = RateLimiterFacade;

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum Deadline {
    Never,
    Instant(Instant),
}

impl From<Deadline> for Option<Instant> {
    fn from(value: Deadline) -> Self {
        match value {
            Deadline::Never => None,
            Deadline::Instant(value) => Some(value),
        }
    }
}

pub enum RateLimiterFacade {
    NoTraffic,
    RateLimiter(SharedTokenBucket),
}

impl RateLimiterFacade {
    pub fn new(rate: RatePerSecond) -> Self {
        match rate {
            RatePerSecond::Block => Self::NoTraffic,
            RatePerSecond::Rate(rate) => Self::RateLimiter(SharedTokenBucket::new(rate)),
        }
    }

    pub async fn rate_limit(self, read_size: usize) -> Self {
        match self {
            RateLimiterFacade::NoTraffic => pending().await,
            RateLimiterFacade::RateLimiter(rate_limiter) => RateLimiterFacade::RateLimiter(
                rate_limiter
                    .rate_limit(read_size.try_into().unwrap_or(u64::MAX))
                    .await,
            ),
        }
    }

    pub fn share(&self) -> Self {
        match self {
            RateLimiterFacade::NoTraffic => RateLimiterFacade::NoTraffic,
            RateLimiterFacade::RateLimiter(shared_token_bucket) => {
                RateLimiterFacade::RateLimiter(shared_token_bucket.share())
            }
        }
    }
}
