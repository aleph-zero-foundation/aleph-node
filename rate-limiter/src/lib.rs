mod rate_limiter;
mod token_bucket;

pub use crate::rate_limiter::{RateLimiter, SleepingRateLimiter};

const LOG_TARGET: &str = "rate-limiter";
