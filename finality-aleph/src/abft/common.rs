use std::{sync::Arc, time::Duration};

use crate::UnitCreationDelay;

// Chosen as a round number large enough so that given the default 200 ms unit creation delay, and the exponential
// slowdown consts below, the time to reach the max round noticeably surpasses the required 7 days. With this
// setting max round should be reached in ~12.5 days.
pub const MAX_ROUNDS: u16 = 10_000;

// Given the default 200 ms unit creation delay, the expected no-slowdown time will be 7500*200/1000/60 = 25 minutes
// which is noticeably longer than the expected 15 minutes of session time.
const EXP_SLOWDOWN_START_ROUND: usize = 7500;
const EXP_SLOWDOWN_MUL: f64 = 1.004;

fn exponential_slowdown(
    t: usize,
    base_delay: f64,
    start_exp_delay: usize,
    exp_base: f64,
) -> Duration {
    // This gives:
    // base_delay, for t <= start_exp_delay,
    // base_delay * exp_base^(t - start_exp_delay), for t > start_exp_delay.
    let delay = if t < start_exp_delay {
        base_delay
    } else {
        let power = t - start_exp_delay;
        base_delay * exp_base.powf(power as f64)
    };
    let delay = delay.round() as u64;
    // the above will make it u64::MAX if it exceeds u64
    Duration::from_millis(delay)
}

pub type DelaySchedule = Arc<dyn Fn(usize) -> Duration + Sync + Send + 'static>;

pub fn unit_creation_delay_fn(unit_creation_delay: UnitCreationDelay) -> DelaySchedule {
    Arc::new(move |t| match t {
        0 => Duration::from_millis(2000),
        _ => exponential_slowdown(
            t,
            unit_creation_delay.0 as f64,
            EXP_SLOWDOWN_START_ROUND,
            EXP_SLOWDOWN_MUL,
        ),
    })
}

// 7 days (as milliseconds)
pub const SESSION_LEN_LOWER_BOUND_MS: u128 = 1000 * 60 * 60 * 24 * 7;
