use tokio::time::{sleep, Duration, Instant};

/// This struct is used for rate limiting as an on-demand ticker. It can be used for ticking
/// at least once `max_timeout` but not more than once every `min_timeout`.
/// Example usage would be to use `wait` method in main select loop and `try_tick` whenever
/// you would like to tick sooner in another branch of select.
pub struct Ticker {
    last_tick: Instant,
    current_timeout: Duration,
    max_timeout: Duration,
    min_timeout: Duration,
}

impl Ticker {
    /// Retruns new Ticker struct. Behaves as if last tick happened during creation of TIcker.
    /// Requires `max_timeout` >= `min_timeout`.
    pub fn new(mut max_timeout: Duration, min_timeout: Duration) -> Self {
        if max_timeout < min_timeout {
            max_timeout = min_timeout;
        };
        Self {
            last_tick: Instant::now(),
            current_timeout: max_timeout,
            max_timeout,
            min_timeout,
        }
    }

    /// Returns whether at least `min_timeout` time elapsed since the last tick.
    /// If `min_timeout` elapsed since the last tick, returns true and records a tick.
    /// If not, returns false and calls to `wait` will return when `min_timeout`
    /// elapses until the next tick.
    pub fn try_tick(&mut self) -> bool {
        let now = Instant::now();
        if now.saturating_duration_since(self.last_tick) >= self.min_timeout {
            self.last_tick = now;
            self.current_timeout = self.max_timeout;
            true
        } else {
            self.current_timeout = self.min_timeout;
            false
        }
    }

    /// Sleeps until next tick should happen.
    /// When enough time elapsed, returns and records a tick.
    pub async fn wait_and_tick(&mut self) {
        let since_last = Instant::now().saturating_duration_since(self.last_tick);
        sleep(self.current_timeout.saturating_sub(since_last)).await;
        self.current_timeout = self.max_timeout;
        self.last_tick = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use tokio::time::{sleep, timeout, Duration};

    use super::Ticker;

    const MAX_TIMEOUT: Duration = Duration::from_millis(700);
    const MIN_TIMEOUT: Duration = Duration::from_millis(100);

    const MAX_TIMEOUT_PLUS: Duration = Duration::from_millis(800);
    const MIN_TIMEOUT_PLUS: Duration = Duration::from_millis(200);

    fn setup_ticker() -> Ticker {
        Ticker::new(MAX_TIMEOUT, MIN_TIMEOUT)
    }

    #[tokio::test]
    async fn try_tick() {
        let mut ticker = setup_ticker();

        assert!(!ticker.try_tick());
        sleep(MIN_TIMEOUT).await;
        assert!(ticker.try_tick());
        assert!(!ticker.try_tick());
    }

    #[tokio::test]
    async fn wait() {
        let mut ticker = setup_ticker();

        assert_ne!(
            timeout(MIN_TIMEOUT_PLUS, ticker.wait_and_tick()).await,
            Ok(())
        );
        assert_eq!(timeout(MAX_TIMEOUT, ticker.wait_and_tick()).await, Ok(()));
    }

    #[tokio::test]
    async fn wait_after_try_tick_true() {
        let mut ticker = setup_ticker();

        assert!(!ticker.try_tick());
        sleep(MIN_TIMEOUT).await;
        assert!(ticker.try_tick());

        assert_ne!(
            timeout(MIN_TIMEOUT_PLUS, ticker.wait_and_tick()).await,
            Ok(())
        );
        assert_eq!(timeout(MAX_TIMEOUT, ticker.wait_and_tick()).await, Ok(()));
    }

    #[tokio::test]
    async fn wait_after_try_tick_false() {
        let mut ticker = setup_ticker();

        assert!(!ticker.try_tick());

        assert_eq!(
            timeout(MIN_TIMEOUT_PLUS, ticker.wait_and_tick()).await,
            Ok(())
        );
        assert_ne!(
            timeout(MIN_TIMEOUT_PLUS, ticker.wait_and_tick()).await,
            Ok(())
        );
        assert_eq!(timeout(MAX_TIMEOUT, ticker.wait_and_tick()).await, Ok(()));
    }

    #[tokio::test]
    async fn try_tick_after_wait() {
        let mut ticker = setup_ticker();

        assert_eq!(
            timeout(MAX_TIMEOUT_PLUS, ticker.wait_and_tick()).await,
            Ok(())
        );

        assert!(!ticker.try_tick());
        sleep(MIN_TIMEOUT).await;
        assert!(ticker.try_tick());
    }
}
