use std::{sync::Arc, time::Duration};

use crate::UnitCreationDelay;

pub const MAX_ROUNDS: u16 = 7000;

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
        _ => exponential_slowdown(t, unit_creation_delay.0 as f64, 5000, 1.005),
    })
}

// 7 days (as milliseconds)
const SESSION_LEN_LOWER_BOUND_MS: u128 = 1000 * 60 * 60 * 24 * 7;

pub fn sanity_check_round_delays(max_rounds: u16, round_delays: DelaySchedule) {
    let delays_ok = sanity_check_round_delays_inner(max_rounds, round_delays);
    assert!(
        delays_ok,
        "Incorrect setting of delays. Make sure the total AlephBFT session time is at least {SESSION_LEN_LOWER_BOUND_MS}ms."
    );
}

fn sanity_check_round_delays_inner(max_rounds: u16, round_delays: DelaySchedule) -> bool {
    let mut total_delay = Duration::from_millis(0);
    for t in 0..=max_rounds {
        total_delay += round_delays(t as usize);
    }
    total_delay.as_millis() > SESSION_LEN_LOWER_BOUND_MS
}

#[test]
fn sanity_check_fails_on_bad_config() {
    let round_delays = unit_creation_delay_fn(UnitCreationDelay(300));
    assert!(!sanity_check_round_delays_inner(5000, round_delays));
}

#[test]
fn sanity_check_passes_on_good_config() {
    let round_delays = unit_creation_delay_fn(UnitCreationDelay(300));
    assert!(sanity_check_round_delays_inner(7000, round_delays));
}
