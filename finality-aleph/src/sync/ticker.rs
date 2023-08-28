use tokio::time::{sleep, Duration, Instant};

enum Mode {
    Normal,
    Rushed,
}

/// This struct is used for rate limiting as an on-demand ticker. It can be used for ticking
/// at most after `max_timeout` but no sooner than after `min_timeout`.
/// Example usage would be to use the `wait` method in main select loop and
/// `try_tick` whenever you would like to tick sooner in another branch of select,
/// resetting whenever the rate limited action actually occurs.
pub struct Ticker {
    last_reset: Instant,
    mode: Mode,
    max_timeout: Duration,
    min_timeout: Duration,
}

impl Ticker {
    /// Returns new Ticker struct. Enforces `max_timeout` >= `min_timeout`.
    pub fn new(mut max_timeout: Duration, min_timeout: Duration) -> Self {
        if max_timeout < min_timeout {
            max_timeout = min_timeout;
        };
        Self {
            last_reset: Instant::now(),
            mode: Mode::Normal,
            max_timeout,
            min_timeout,
        }
    }

    /// Returns whether at least `min_timeout` time elapsed since the last reset.
    /// If it has not, the next call to `wait_and_tick` will return when `min_timeout` elapses.
    pub fn try_tick(&mut self) -> bool {
        let now = Instant::now();
        if now.saturating_duration_since(self.last_reset) >= self.min_timeout {
            self.mode = Mode::Normal;
            true
        } else {
            self.mode = Mode::Rushed;
            false
        }
    }

    /// Sleeps until next tick should happen.
    /// Returns when enough time elapsed.
    /// Returns whether `max_timeout` elapsed since the last reset, and if so also resets.
    ///
    /// # Cancel safety
    ///
    /// This method is cancellation safe.
    pub async fn wait_and_tick(&mut self) -> bool {
        self.wait_current_timeout().await;
        match self.since_reset() > self.max_timeout {
            true => {
                self.reset();
                true
            }
            false => {
                self.mode = Mode::Normal;
                false
            }
        }
    }

    /// Reset the ticker, making it time from the moment of this call.
    /// Behaves as if it was just created with the same parametres.
    pub fn reset(&mut self) {
        self.last_reset = Instant::now();
        self.mode = Mode::Normal;
    }

    fn since_reset(&self) -> Duration {
        Instant::now().saturating_duration_since(self.last_reset)
    }

    async fn wait_current_timeout(&self) {
        let sleep_time = match self.mode {
            Mode::Normal => self.max_timeout,
            Mode::Rushed => self.min_timeout,
        }
        .saturating_sub(self.since_reset());
        sleep(sleep_time).await;
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
        sleep(MIN_TIMEOUT_PLUS).await;
        assert!(ticker.try_tick());
        assert!(ticker.try_tick());
    }

    #[tokio::test]
    async fn plain_wait() {
        let mut ticker = setup_ticker();

        assert!(matches!(
            timeout(MIN_TIMEOUT_PLUS, ticker.wait_and_tick()).await,
            Err(_)
        ));
        assert_eq!(timeout(MAX_TIMEOUT, ticker.wait_and_tick()).await, Ok(true));
        assert!(matches!(
            timeout(MIN_TIMEOUT_PLUS, ticker.wait_and_tick()).await,
            Err(_)
        ));
    }

    #[tokio::test]
    async fn wait_after_try_tick_true() {
        let mut ticker = setup_ticker();

        assert!(!ticker.try_tick());
        sleep(MIN_TIMEOUT).await;
        assert!(ticker.try_tick());

        assert!(matches!(
            timeout(MIN_TIMEOUT_PLUS, ticker.wait_and_tick()).await,
            Err(_)
        ));
        assert_eq!(timeout(MAX_TIMEOUT, ticker.wait_and_tick()).await, Ok(true));
    }

    #[tokio::test]
    async fn wait_after_try_tick_false() {
        let mut ticker = setup_ticker();

        assert!(!ticker.try_tick());

        assert_eq!(
            timeout(MIN_TIMEOUT_PLUS, ticker.wait_and_tick()).await,
            Ok(false)
        );
        assert!(matches!(
            timeout(MIN_TIMEOUT_PLUS, ticker.wait_and_tick()).await,
            Err(_)
        ));
        assert_eq!(timeout(MAX_TIMEOUT, ticker.wait_and_tick()).await, Ok(true));
    }

    #[tokio::test]
    async fn try_tick_after_wait() {
        let mut ticker = setup_ticker();

        assert_eq!(
            timeout(MAX_TIMEOUT_PLUS, ticker.wait_and_tick()).await,
            Ok(true)
        );

        assert!(!ticker.try_tick());
    }

    #[tokio::test]
    async fn wait_after_late_reset() {
        let mut ticker = setup_ticker();

        assert_eq!(
            timeout(MAX_TIMEOUT_PLUS, ticker.wait_and_tick()).await,
            Ok(true)
        );

        ticker.reset();
        assert!(matches!(
            timeout(MIN_TIMEOUT_PLUS, ticker.wait_and_tick()).await,
            Err(_)
        ));
        assert_eq!(
            timeout(MAX_TIMEOUT_PLUS, ticker.wait_and_tick()).await,
            Ok(true)
        );
    }

    #[tokio::test]
    async fn wait_after_early_reset() {
        let mut ticker = setup_ticker();

        sleep(MIN_TIMEOUT).await;
        assert!(ticker.try_tick());

        ticker.reset();
        assert!(matches!(
            timeout(MIN_TIMEOUT_PLUS, ticker.wait_and_tick()).await,
            Err(_)
        ));
        assert_eq!(
            timeout(MAX_TIMEOUT_PLUS, ticker.wait_and_tick()).await,
            Ok(true)
        );
    }

    #[tokio::test]
    async fn try_tick_after_reset() {
        let mut ticker = setup_ticker();

        assert_eq!(
            timeout(MAX_TIMEOUT_PLUS, ticker.wait_and_tick()).await,
            Ok(true)
        );

        ticker.reset();
        assert!(!ticker.try_tick());
        sleep(MIN_TIMEOUT).await;
        assert!(ticker.try_tick());
    }
}
