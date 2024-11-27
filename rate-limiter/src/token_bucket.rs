use std::{
    cmp::min,
    num::NonZeroU64,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use futures::{future::pending, Future, FutureExt};
use log::trace;
use tokio::time::sleep;

use crate::{NonZeroRatePerSecond, LOG_TARGET, MIN};

/// Returns a non-decreasing values of type [std::time::Instant].
pub trait TimeProvider {
    fn now(&self) -> Instant;
}

/// Default implementation of the [TimeProvider] trait using [tokio::time].
#[derive(Clone, Default)]
pub struct TokioTimeProvider;

impl TimeProvider for TokioTimeProvider {
    fn now(&self) -> Instant {
        // We use [tokio::time::Instant] in order to be consistent with the
        // implementation of our [TokioSleepUntil] below. At the time of
        // writing, [tokio::time::Instant] simply wraps [std::time::Instant].
        tokio::time::Instant::now().into()
    }
}

/// Implementation of a sleep mechanism that doesn't block an executor thread of an async runtime.
/// Implementations should be cancellation-safe.
pub trait SleepUntil {
    fn sleep_until(&mut self, instant: Instant) -> impl Future<Output = ()> + Send;
}

/// Default implementation of the [SleepUntil] trait using [tokio::time].
#[derive(Clone, Default)]
pub struct TokioSleepUntil;

impl SleepUntil for TokioSleepUntil {
    async fn sleep_until(&mut self, instant: Instant) {
        tokio::time::sleep_until(instant.into()).await;
    }
}

/// Implementation of the `Token Bucket` algorithm for the purpose of rate-limiting access to some abstract resource, e.g. data received via network.
#[derive(Clone)]
struct TokenBucket<T = TokioTimeProvider> {
    last_update: Instant,
    rate_per_second: NonZeroU64,
    requested: u64,
    time_provider: T,
}

impl<T> std::fmt::Debug for TokenBucket<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenBucket")
            .field("last_update", &self.last_update)
            .field("rate_per_second", &self.rate_per_second)
            .field("requested", &self.requested)
            .finish()
    }
}

impl TokenBucket {
    /// Constructs an instance of [`TokenBucket`] with given target rate-per-second.
    pub fn new(rate_per_second: NonZeroRatePerSecond) -> Self {
        Self::new_internal(rate_per_second, Default::default())
    }
}

impl<TP> TokenBucket<TP>
where
    TP: TimeProvider,
{
    fn new_internal(rate_per_second: NonZeroRatePerSecond, time_provider: TP) -> Self {
        let now = time_provider.now();
        Self {
            time_provider,
            last_update: now,
            rate_per_second: rate_per_second.into(),
            requested: NonZeroU64::from(rate_per_second).into(),
        }
    }

    fn upper_bound_of_tokens(&self) -> u64 {
        self.rate_per_second.into()
    }

    fn available(&self) -> Option<u64> {
        (self.requested <= self.upper_bound_of_tokens())
            .then(|| self.upper_bound_of_tokens() - self.requested)
    }

    fn account_requested_tokens(&mut self, requested: u64) {
        self.requested = self.requested.saturating_add(requested);
    }

    fn calculate_delay(&self) -> Option<Instant> {
        if self.available().is_some() {
            return None;
        }

        let scheduled_for_later = self.requested - self.upper_bound_of_tokens();
        let delay_millis = scheduled_for_later
            .saturating_mul(1_000)
            .saturating_div(self.rate_per_second.into());

        Some(self.last_update + Duration::from_millis(delay_millis))
    }

    fn update_tokens(&mut self) {
        let now = self.time_provider.now();
        assert!(
            now >= self.last_update,
            "Provided value for `now` should be at least equal to `self.last_update`: now = {:#?} self.last_update = {:#?}.",
            now,
            self.last_update
        );

        let time_since_last_update = now.duration_since(self.last_update);
        self.last_update = now;
        let new_units = time_since_last_update
            .as_millis()
            .saturating_mul(u64::from(self.rate_per_second).into())
            .saturating_div(1_000)
            .try_into()
            .unwrap_or(u64::MAX);
        self.requested = self.requested.saturating_sub(new_units);
    }

    /// Gets current rate in bits-per-second.
    pub fn rate(&self) -> NonZeroRatePerSecond {
        self.rate_per_second.into()
    }

    /// Sets a rate in bits-per-second.
    pub fn set_rate(&mut self, rate_per_second: NonZeroRatePerSecond) {
        // We need to update our tokens till now using previous rate.
        self.update_tokens();
        // We need to convert all left tokens to format compatible with the new rate.
        let available = self.available();
        let previous_rate_per_second = self.rate_per_second.get();
        self.rate_per_second = rate_per_second.into();
        if let Some(available) = available {
            let max_for_available = self.upper_bound_of_tokens();
            let available_after_rate_update = min(available, max_for_available);
            self.requested = self.rate_per_second.get() - available_after_rate_update;
        } else {
            self.requested = self.requested - previous_rate_per_second + self.rate_per_second.get();
        }
    }

    /// Calculates amount of time by which we should delay next call to some governed resource in order to satisfy
    /// configured rate limit.
    pub fn rate_limit(&mut self, requested: u64) -> Option<Instant> {
        trace!(
            target: LOG_TARGET,
            "TokenBucket called for {requested} of requested bytes. Internal state: {self:?}.",
        );
        let now_available = self.available().unwrap_or(0);
        if now_available < requested {
            self.update_tokens()
        }
        self.account_requested_tokens(requested);
        let delay = self.calculate_delay();
        trace!(
            target: LOG_TARGET,
            "TokenBucket calculated delay after receiving a request of {requested}: {delay:?}.",
        );
        delay
    }
}

/// Determines how often instances of [SharedBandwidthManager] should check if their allocated bandwidth has changed.
const BANDWIDTH_CHECK_INTERVAL: Duration = Duration::from_millis(250);

/// Implementation of the bandwidth sharing strategy that attempts to assign equal portion of the total bandwidth to all active
/// consumers of that bandwidth.
pub struct SharedBandwidthManager {
    max_rate: NonZeroRatePerSecond,
    peers_count: Arc<AtomicU64>,
    already_requested: Option<NonZeroRatePerSecond>,
}

impl SharedBandwidthManager {
    /// Constructs a new instance of [SharedBandwidthManager] configured with a given rate that will be shared between all
    /// calling consumers (clones of this instance).
    pub fn new(max_rate: NonZeroRatePerSecond) -> Self {
        Self {
            max_rate,
            peers_count: Arc::new(AtomicU64::new(0)),
            already_requested: None,
        }
    }

    pub fn share(&self) -> Self {
        Self {
            max_rate: self.max_rate,
            peers_count: self.peers_count.clone(),
            already_requested: None,
        }
    }

    fn calculate_bandwidth(&mut self, active_children: Option<u64>) -> NonZeroRatePerSecond {
        let active_children =
            active_children.unwrap_or_else(|| self.peers_count.load(Ordering::SeqCst));
        let rate = u64::from(self.max_rate) / active_children;
        NonZeroU64::try_from(rate)
            .map(NonZeroRatePerSecond::from)
            .unwrap_or(MIN)
    }

    /// Allocate part of the shared bandwidth.
    pub fn request_bandwidth(&mut self) -> NonZeroRatePerSecond {
        let active_children = (self.already_requested.is_none())
            .then(|| 1 + self.peers_count.fetch_add(1, Ordering::SeqCst));
        let rate = self.calculate_bandwidth(active_children);
        self.already_requested = Some(rate);
        rate
    }

    /// Notify this manager that we no longer use our allocated bandwidth and so
    /// it can be immediately shared with other active consumers.
    pub fn notify_idle(&mut self) {
        if self.already_requested.take().is_some() {
            self.peers_count.fetch_sub(1, Ordering::SeqCst);
        }
    }

    /// Awaits for a notification about some change to previously allocated rate. For performance reasons, it simply actively
    /// queries for all active peers in a looped manner on every interval of [BANDWIDTH_CHECK_INTERVAL]. Alternative solutions
    /// could use a mechanism similar to [tokio::sync::watch], but our tests showed that such solutions perform rather poorly
    /// compared to this approach.
    pub async fn bandwidth_changed(&mut self) -> NonZeroRatePerSecond {
        let Some(previous_rate) = self.already_requested else {
            return pending().await;
        };
        let mut rate = self.calculate_bandwidth(None);
        while rate == previous_rate {
            sleep(BANDWIDTH_CHECK_INTERVAL).await;
            rate = self.calculate_bandwidth(None);
        }
        self.already_requested = Some(rate);
        rate
    }
}

/// Wrapper around the [TokenBucket] that allows to conveniently manage its
/// internal bit-rate and manages its idle/sleep state in order to accurately
/// satisfy its rate-limit.
#[derive(Clone)]
struct AsyncTokenBucket<TP = TokioTimeProvider, SU = TokioSleepUntil> {
    token_bucket: TokenBucket<TP>,
    next_deadline: Option<Instant>,
    sleep_until: SU,
}

impl<TP, SU> AsyncTokenBucket<TP, SU>
where
    TP: TimeProvider,
{
    /// Constructs an instance of [AsyncTokenBucket] using given [TokenBucket]
    /// and implementation of the [SleepUntil] trait.
    pub fn new(token_bucket: TokenBucket<TP>, sleep_until: SU) -> Self {
        Self {
            token_bucket,
            next_deadline: None,
            sleep_until,
        }
    }

    /// Accounts `requested` units. A next call to [AsyncTokenBucket::wait] will
    /// account these units while calculating necessary delay.
    pub fn rate_limit(&mut self, requested: u64) {
        self.next_deadline = TokenBucket::rate_limit(&mut self.token_bucket, requested);
    }

    /// Sets rate of this limiter and updates the required delay accordingly.
    pub fn set_rate(&mut self, rate: NonZeroRatePerSecond) {
        if self.token_bucket.rate() != rate {
            self.token_bucket.set_rate(rate);
            self.next_deadline = self.token_bucket.rate_limit(0);
        }
    }

    /// Makes current task idle in order to fulfill configured rate.
    pub async fn wait(&mut self)
    where
        TP: TimeProvider + Send,
        SU: SleepUntil + Send,
    {
        if let Some(deadline) = self.next_deadline {
            self.sleep_until.sleep_until(deadline).await;
            self.next_deadline = None;
        }
    }
}

/// [SharedTokenBucket] allows to share a given amount of bandwidth between multiple instances of [TokenBucket]. Each time an
/// instance requests to share the bandwidth, it is given a fair share of it, i.e. `all available bandwidth / # of active
/// instances`. All instances, that previously acquired some share of the bandwidth, are actively querying (with some predefined
/// interval) for changes in their allocated share. Alternatively to this polling procedure, we could devise a method where on
/// each new request for sharing the bandwidth, we actively query every active instance to confirm a change before we allocate
/// it for a new peer. This would provide each requesting instance a more accurate share of the bandwidth, but it would also
/// have a huge negative impact on performance. We believe, current solution is a good compromise between accuracy and
/// performance. For this querying strategy, in worst case, utilized bandwidth should be equal to `bandwidth * (1 + 1/2 + ... +
/// 1/n) ≈ bandwidth * (ln n + O(1))`. This can happen when each instance of [TokenBucket] tries to spend slightly more data
/// than its initially acquired bandwidth, but small enough so none of them other instances receives a notification about
/// ongoing bandwidth change.
pub struct SharedTokenBucket<TP = TokioTimeProvider, SU = TokioSleepUntil> {
    shared_bandwidth: SharedBandwidthManager,
    rate_limiter: AsyncTokenBucket<TP, SU>,
    need_to_notify_parent: bool,
}

impl SharedTokenBucket {
    /// Constructs a new instance of [SharedTokenBucket] using a given `rate` as the maximal amount of bandwidth that will be
    /// shared between all of its cloned instances.
    pub fn new(rate: NonZeroRatePerSecond) -> Self {
        let token_bucket = TokenBucket::new(rate);
        let sleep_until = TokioSleepUntil;
        let rate_limiter = AsyncTokenBucket::new(token_bucket, sleep_until);
        Self::new_internal(rate, rate_limiter)
    }
}

impl<TP, SU> SharedTokenBucket<TP, SU> {
    fn new_internal(rate: NonZeroRatePerSecond, rate_limiter: AsyncTokenBucket<TP, SU>) -> Self {
        Self {
            shared_bandwidth: SharedBandwidthManager::new(rate),
            rate_limiter,
            need_to_notify_parent: false,
        }
    }

    pub fn share(&self) -> Self
    where
        TP: Clone,
        SU: Clone,
    {
        Self {
            shared_bandwidth: self.shared_bandwidth.share(),
            rate_limiter: self.rate_limiter.clone(),
            need_to_notify_parent: false,
        }
    }

    fn request_bandwidth(&mut self) -> NonZeroRatePerSecond {
        self.need_to_notify_parent = true;
        self.shared_bandwidth.request_bandwidth()
    }

    fn notify_idle(&mut self) {
        if self.need_to_notify_parent {
            self.shared_bandwidth.notify_idle();
            self.need_to_notify_parent = false;
        }
    }

    /// Executes the rate-limiting strategy and delays execution in order to satisfy configured rate-limit.
    pub async fn rate_limit(mut self, requested: u64) -> Self
    where
        TP: TimeProvider + Send,
        SU: SleepUntil + Send,
    {
        let rate = self.request_bandwidth();
        self.rate_limiter.set_rate(rate);

        self.rate_limiter.rate_limit(requested);

        loop {
            futures::select! {
                _ = self.rate_limiter.wait().fuse() => {
                    self.notify_idle();
                    return self;
                },
                rate = self.shared_bandwidth.bandwidth_changed().fuse() => {
                    self.rate_limiter.set_rate(rate);
                },
            }
        }
    }
}

impl<TP, SU> Drop for SharedTokenBucket<TP, SU> {
    fn drop(&mut self) {
        self.notify_idle();
    }
}

#[cfg(test)]
mod tests {
    use std::{
        cmp::{max, min},
        iter::repeat,
        sync::Arc,
        task::Poll,
        thread,
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };

    use futures::{
        future::{pending, poll_fn, BoxFuture, Future},
        pin_mut,
        stream::FuturesOrdered,
        StreamExt,
    };
    use parking_lot::Mutex;

    use super::{SharedBandwidthManager, SleepUntil, TimeProvider, TokenBucket};
    use crate::token_bucket::{AsyncTokenBucket, NonZeroRatePerSecond, SharedTokenBucket};

    impl<F> TimeProvider for F
    where
        F: Fn() -> Instant,
    {
        fn now(&self) -> Instant {
            self()
        }
    }

    impl TimeProvider for Arc<Box<dyn TimeProvider + Send + Sync + 'static>> {
        fn now(&self) -> Instant {
            self.as_ref().now()
        }
    }

    #[tokio::test]
    async fn basic_checks_of_shared_bandwidth_manager() {
        let rate = 10.try_into().expect("10 > 0 qed");
        let mut bandwidth_share = SharedBandwidthManager::new(rate);
        let mut cloned_bandwidth_share = bandwidth_share.share();
        let mut another_cloned_bandwidth_share = cloned_bandwidth_share.share();

        // only one consumer, so it should get whole bandwidth
        assert_eq!(bandwidth_share.request_bandwidth(), rate);

        // since other instances did not request for bandwidth, they should not receive notification that it has changed
        let poll_result = poll_fn(|cx| {
            let future = cloned_bandwidth_share.bandwidth_changed();
            pin_mut!(future);
            Poll::Ready(Future::poll(future, cx))
        })
        .await;
        assert_eq!(poll_result, Poll::Pending);

        let poll_result = poll_fn(|cx| {
            let future = another_cloned_bandwidth_share.bandwidth_changed();
            pin_mut!(future);
            Poll::Ready(Future::poll(future, cx))
        })
        .await;
        assert_eq!(poll_result, Poll::Pending);

        // two consumers should equally divide the bandwidth
        let rate = 5.try_into().expect("5 > 0 qed");
        assert_eq!(cloned_bandwidth_share.request_bandwidth(), rate);
        assert_eq!(bandwidth_share.bandwidth_changed().await, rate);

        // similarly when there are three of them
        let bandwidth: u64 = another_cloned_bandwidth_share.request_bandwidth().into();
        let another_bandwidth: u64 = bandwidth_share.bandwidth_changed().await.into();
        let yet_another_bandwidth: u64 = cloned_bandwidth_share.bandwidth_changed().await.into();

        assert!((3..4).contains(&bandwidth));
        assert!((3..4).contains(&another_bandwidth));
        assert!((3..4).contains(&yet_another_bandwidth));

        assert!((9..10).contains(&(bandwidth + another_bandwidth + yet_another_bandwidth)));

        // all consumers should be notified after one of them become idle
        let rate = 5.try_into().expect("5 > 0 qed");
        another_cloned_bandwidth_share.notify_idle();
        assert_eq!(cloned_bandwidth_share.bandwidth_changed().await, rate);
        assert_eq!(bandwidth_share.bandwidth_changed().await, rate);
    }

    /// Allows to treat [TokenBucket] and [SharedTokenBucket] in similar fashion in our tests.
    trait RateLimiter: Sized {
        async fn rate_limit(self, requested: u64) -> (Self, Option<Instant>);
    }

    impl<TP> RateLimiter for TokenBucket<TP>
    where
        TP: TimeProvider,
    {
        async fn rate_limit(mut self, requested: u64) -> (Self, Option<Instant>) {
            let delay = TokenBucket::rate_limit(&mut self, requested);
            (self, delay)
        }
    }

    type TracingRateLimiter<TP> = SharedTokenBucket<TP, SharedTracingSleepUntil>;

    impl<TP> RateLimiter for TracingRateLimiter<TP>
    where
        TP: TimeProvider + Send,
    {
        async fn rate_limit(mut self, requested: u64) -> (Self, Option<Instant>) {
            let last_sleep_deadline = self.rate_limiter.sleep_until.last_deadline.clone();
            let time_before = *last_sleep_deadline.lock();
            self = self.rate_limit(requested).await;
            let time_after = *last_sleep_deadline.lock();
            (
                self,
                (time_before != time_after).then_some(time_after).flatten(),
            )
        }
    }

    impl<TP, SU> From<(NonZeroRatePerSecond, TP, SU)> for TokenBucket<TP>
    where
        TP: TimeProvider,
    {
        fn from((rate, time_provider, _): (NonZeroRatePerSecond, TP, SU)) -> Self {
            TokenBucket::new_internal(rate, time_provider)
        }
    }

    impl<TP, SU> From<(NonZeroRatePerSecond, TP, SU)> for SharedTokenBucket<TP, SU>
    where
        TP: TimeProvider,
    {
        fn from((rate, time_provider, sleep_until): (NonZeroRatePerSecond, TP, SU)) -> Self {
            let token_bucket = TokenBucket::new_internal(rate, time_provider);
            let rate_limiter = AsyncTokenBucket::new(token_bucket, sleep_until);
            Self::new_internal(rate, rate_limiter)
        }
    }

    #[derive(Clone)]
    struct SharedTracingSleepUntil {
        pub last_deadline: Arc<Mutex<Option<Instant>>>,
    }

    impl SharedTracingSleepUntil {
        pub fn new() -> Self {
            Self {
                last_deadline: Arc::new(Mutex::new(None)),
            }
        }
    }

    impl SleepUntil for SharedTracingSleepUntil {
        async fn sleep_until(&mut self, instant: Instant) {
            let mut last_instant = self.last_deadline.lock();
            *last_instant = max(*last_instant, Some(instant));
        }
    }

    #[tokio::test]
    async fn rate_limiter_sanity_check() {
        token_bucket_sanity_check_test::<TokenBucket<_>>().await;
        token_bucket_sanity_check_test::<TracingRateLimiter<_>>().await
    }

    async fn token_bucket_sanity_check_test<RL>()
    where
        RL: RateLimiter
            + From<(
                NonZeroRatePerSecond,
                Arc<Box<dyn TimeProvider + Send + Sync>>,
                SharedTracingSleepUntil,
            )>,
    {
        let limit_per_second = 10.try_into().expect("10 > 0 qed");
        let now = Instant::now();
        let time_to_return = Arc::new(parking_lot::RwLock::new(now));
        let time_provider = time_to_return.clone();
        let time_provider: Box<dyn TimeProvider + Send + Sync> =
            Box::new(move || *time_provider.read());
        let rate_limiter = RL::from((
            limit_per_second,
            Arc::new(time_provider),
            SharedTracingSleepUntil::new(),
        ));

        *time_to_return.write() = now + Duration::from_secs(1);
        let (rate_limiter, deadline) = rate_limiter.rate_limit(9).await;
        assert!(deadline.is_none());

        *time_to_return.write() = now + Duration::from_secs(1);
        let (rate_limiter, deadline) = rate_limiter.rate_limit(12).await;
        assert!(deadline.is_some());

        *time_to_return.write() = now + Duration::from_secs(3);
        let (_, deadline) = rate_limiter.rate_limit(8).await;
        assert!(deadline.is_none());
    }

    #[tokio::test]
    async fn no_slowdown_while_within_rate_limit() {
        no_slowdown_while_within_rate_limit_test::<TokenBucket<_>>().await;
        no_slowdown_while_within_rate_limit_test::<TracingRateLimiter<_>>().await;
    }

    async fn no_slowdown_while_within_rate_limit_test<RL>()
    where
        RL: RateLimiter
            + From<(
                NonZeroRatePerSecond,
                Arc<Box<dyn TimeProvider + Send + Sync>>,
                SharedTracingSleepUntil,
            )>,
    {
        let limit_per_second = 10.try_into().expect("10 > 0 qed");
        let now = Instant::now();
        let time_to_return = Arc::new(parking_lot::RwLock::new(now));
        let time_provider = time_to_return.clone();
        let time_provider: Box<dyn TimeProvider + Send + Sync> =
            Box::new(move || *time_provider.read());
        let sleep_until = SharedTracingSleepUntil::new();
        let rate_limiter = RL::from((limit_per_second, Arc::new(time_provider), sleep_until));

        *time_to_return.write() = now + Duration::from_secs(1);
        let (rate_limiter, deadline) = rate_limiter.rate_limit(9).await;
        assert_eq!(deadline, None);

        *time_to_return.write() = now + Duration::from_secs(2);
        let (rate_limiter, deadline) = rate_limiter.rate_limit(5).await;
        assert_eq!(deadline, None);

        *time_to_return.write() = now + Duration::from_secs(3);
        let (rate_limiter, deadline) = rate_limiter.rate_limit(1).await;
        assert_eq!(deadline, None);

        *time_to_return.write() = now + Duration::from_secs(3);
        let (_, deadline) = rate_limiter.rate_limit(9).await;
        assert_eq!(deadline, None);
    }

    #[tokio::test]
    async fn slowdown_when_limit_reached_token_bucket() {
        slowdown_when_limit_reached_test::<TokenBucket<_>>().await;
        slowdown_when_limit_reached_test::<TracingRateLimiter<_>>().await
    }

    async fn slowdown_when_limit_reached_test<RL>()
    where
        RL: RateLimiter
            + From<(
                NonZeroRatePerSecond,
                Arc<Box<dyn TimeProvider + Send + Sync>>,
                SharedTracingSleepUntil,
            )>,
    {
        let limit_per_second = 10.try_into().expect("10 > 0 qed");
        let now = Instant::now();
        let time_to_return = Arc::new(parking_lot::RwLock::new(now));
        let time_provider = time_to_return.clone();
        let time_provider: Box<dyn TimeProvider + Send + Sync> =
            Box::new(move || *time_provider.read());
        let rate_limiter = RL::from((
            limit_per_second,
            Arc::new(time_provider),
            SharedTracingSleepUntil::new(),
        ));

        *time_to_return.write() = now;
        let (rate_limiter, deadline) = rate_limiter.rate_limit(10).await;
        assert_eq!(deadline, Some(now + Duration::from_secs(1)));

        // we should wait some time after reaching the limit
        *time_to_return.write() = now + Duration::from_secs(1);
        let (rate_limiter, deadline) = rate_limiter.rate_limit(1).await;
        assert!(deadline.is_some());

        *time_to_return.write() = now + Duration::from_secs(1);
        let (_, deadline) = rate_limiter.rate_limit(19).await;
        assert_eq!(
            deadline,
            Some(now + Duration::from_secs(3)),
            "we should wait exactly 2 seconds"
        );
    }

    #[tokio::test]
    async fn buildup_tokens_but_no_more_than_limit_of_token_bucket() {
        buildup_tokens_but_no_more_than_limit_test::<TokenBucket<_>>().await;
        buildup_tokens_but_no_more_than_limit_test::<TracingRateLimiter<_>>().await
    }

    async fn buildup_tokens_but_no_more_than_limit_test<RL>()
    where
        RL: RateLimiter
            + From<(
                NonZeroRatePerSecond,
                Arc<Box<dyn TimeProvider + Send + Sync>>,
                SharedTracingSleepUntil,
            )>,
    {
        let limit_per_second = 10.try_into().expect("10 > 0 qed");
        let now = Instant::now();
        let time_to_return = Arc::new(parking_lot::RwLock::new(now));
        let time_provider = time_to_return.clone();
        let time_provider: Box<dyn TimeProvider + Send + Sync> =
            Box::new(move || *time_provider.read());
        let rate_limiter = RL::from((
            limit_per_second,
            time_provider.into(),
            SharedTracingSleepUntil::new(),
        ));

        *time_to_return.write() = now + Duration::from_secs(2);
        let (rate_limiter, deadline) = rate_limiter.rate_limit(10).await;
        assert_eq!(deadline, None);

        *time_to_return.write() = now + Duration::from_secs(10);
        let (rate_limiter, deadline) = rate_limiter.rate_limit(40).await;
        assert_eq!(
            deadline,
            Some(now + Duration::from_secs(10) + Duration::from_secs(3)),
        );

        *time_to_return.write() = now + Duration::from_secs(11);
        let (_, deadline) = rate_limiter.rate_limit(40).await;
        assert_eq!(
            deadline,
            Some(now + Duration::from_secs(11) + Duration::from_secs(6))
        );
    }

    #[tokio::test]
    async fn multiple_calls_buildup_wait_time() {
        multiple_calls_buildup_wait_time_test::<TokenBucket<_>>().await;
        multiple_calls_buildup_wait_time_test::<TracingRateLimiter<_>>().await
    }

    async fn multiple_calls_buildup_wait_time_test<RL>()
    where
        RL: RateLimiter
            + From<(
                NonZeroRatePerSecond,
                Arc<Box<dyn TimeProvider + Send + Sync>>,
                SharedTracingSleepUntil,
            )>,
    {
        let limit_per_second = 10.try_into().expect("10 > 0 qed");
        let now = Instant::now();
        let time_to_return = Arc::new(parking_lot::RwLock::new(now));
        let time_provider = time_to_return.clone();
        let time_provider: Box<dyn TimeProvider + Send + Sync> =
            Box::new(move || *time_provider.read());
        let rate_limiter = RL::from((
            limit_per_second,
            time_provider.into(),
            SharedTracingSleepUntil::new(),
        ));

        *time_to_return.write() = now + Duration::from_secs(3);
        let (rate_limiter, deadline) = rate_limiter.rate_limit(10).await;
        assert_eq!(deadline, None);

        *time_to_return.write() = now + Duration::from_secs(3);
        let (rate_limiter, deadline) = rate_limiter.rate_limit(10).await;
        assert_eq!(deadline, Some(now + Duration::from_secs(4)));

        *time_to_return.write() = now + Duration::from_secs(3);
        let (rate_limiter, deadline) = rate_limiter.rate_limit(10).await;
        assert_eq!(
            deadline,
            Some(now + Duration::from_secs(4) + Duration::from_secs(1))
        );

        *time_to_return.write() = now + Duration::from_secs(3);
        let (_, deadline) = rate_limiter.rate_limit(50).await;
        assert_eq!(
            deadline,
            Some(now + Duration::from_secs(4) + Duration::from_secs(6))
        );
    }

    /// It allows to wait for a change of the allocated bandwidth, i.e. it can wait for two (or more) calls of
    /// [SleepUntil::sleep_until] that uses two different values of its `instant` argument.
    struct SleepUntilAfterChange<SU> {
        wrapped: SU,
        last_value: Option<Instant>,
        changes_counter: usize,
    }

    impl<SU> SleepUntilAfterChange<SU> {
        pub fn new(sleep_until: SU) -> Self {
            Self {
                wrapped: sleep_until,
                last_value: None,
                changes_counter: 0,
            }
        }

        pub fn set_number_of_changes_to_wait(&mut self, changes_counter: usize) {
            self.changes_counter = changes_counter;
            self.last_value = None;
        }
    }

    impl<SU> Clone for SleepUntilAfterChange<SU>
    where
        SU: Clone,
    {
        fn clone(&self) -> Self {
            Self {
                wrapped: self.wrapped.clone(),
                last_value: None,
                changes_counter: self.changes_counter,
            }
        }
    }

    impl<SU> SleepUntil for SleepUntilAfterChange<SU>
    where
        SU: SleepUntil + Send,
    {
        async fn sleep_until(&mut self, instant: Instant) {
            let last_value = self.last_value.get_or_insert(instant);
            if *last_value != instant {
                self.changes_counter = self.changes_counter.saturating_sub(1);
            }
            if self.changes_counter == 0 {
                self.wrapped.sleep_until(instant).await
            } else {
                pending().await
            }
        }
    }

    #[tokio::test]
    async fn two_peers_can_share_bandwidth() {
        let limit_per_second = 10.try_into().expect("10 > 0 qed");
        let initial_time = Instant::now();
        let time_to_return = Arc::new(Mutex::new(initial_time));

        let current_time = time_to_return.clone();
        let current_time_clone = time_to_return.clone();

        let time_provider: Arc<Box<dyn TimeProvider + Send + Sync>> =
            Arc::new(Box::new(move || *time_to_return.lock()));

        let sleep_until = SharedTracingSleepUntil::new();
        let last_deadline = sleep_until.last_deadline.clone();
        let last_deadline_clone = last_deadline.clone();
        let sleep_until = SleepUntilAfterChange::new(sleep_until);

        let barrier = Arc::new(tokio::sync::RwLock::new(tokio::sync::Barrier::new(2)));
        let second_barrier = barrier.clone();

        let mut rate_limiter =
            SharedTokenBucket::<_, _>::from((limit_per_second, time_provider, sleep_until));
        let mut rate_limiter_cloned = rate_limiter.share();

        let total_data_sent = thread::scope(|s| {
            let first_handle = s.spawn(|| {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_time()
                    .build()
                    .unwrap();

                runtime.block_on(async move {
                    barrier.read().await.wait().await;

                    rate_limiter = rate_limiter.rate_limit(11).await;
                    rate_limiter
                        .rate_limiter
                        .sleep_until
                        .set_number_of_changes_to_wait(1);

                    {
                        let last_deadline = last_deadline.lock();
                        let mut current_time = current_time.lock();
                        *current_time = last_deadline.unwrap_or(*current_time);
                    }

                    barrier.read().await.wait().await;

                    rate_limiter.rate_limit(30).await;

                    {
                        let last_deadline = last_deadline.lock();
                        let mut current_time = current_time.lock();
                        *current_time = last_deadline.unwrap_or(*current_time);
                    }
                });
                11 + 30
            });

            let second_handle = s.spawn(|| {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_time()
                    .build()
                    .unwrap();

                runtime.block_on(async {
                    second_barrier.read().await.wait().await;

                    rate_limiter_cloned = rate_limiter_cloned.rate_limit(13).await;
                    rate_limiter_cloned
                        .rate_limiter
                        .sleep_until
                        .set_number_of_changes_to_wait(1);

                    {
                        let last_deadline = last_deadline_clone.lock();
                        let mut current_time = current_time_clone.lock();
                        *current_time = last_deadline.unwrap_or(*current_time);
                    }

                    second_barrier.read().await.wait().await;

                    rate_limiter_cloned.rate_limit(25).await;

                    {
                        let last_deadline = last_deadline_clone.lock();
                        let mut current_time = current_time_clone.lock();
                        *current_time = last_deadline.unwrap_or(*current_time);
                    }
                });
                13 + 25
            });
            let total_data_sent: u128 = first_handle
                .join()
                .expect("first thread should finish without errors")
                + second_handle
                    .join()
                    .expect("second thread should finish without errors");

            total_data_sent
        });
        let duration = last_deadline_clone.lock().expect("we should sleep a bit") - initial_time;
        let rate = total_data_sent * 1000 / duration.as_millis();
        assert!(
            rate.abs_diff(10) <= 5,
            "calculated bandwidth should be within some error bounds: rate = {rate}; duration = {duration:?}"
        );
    }

    #[tokio::test]
    async fn single_peer_can_use_whole_bandwidth() {
        let limit_per_second = 10.try_into().expect("10 > 0 qed");
        let now = Instant::now();
        let time_to_return = Arc::new(parking_lot::RwLock::new(now));
        let time_provider = time_to_return.clone();
        let time_provider: Arc<Box<dyn TimeProvider + Send + Sync>> =
            Arc::new(Box::new(move || *time_provider.read()));

        let rate_limiter = TracingRateLimiter::<_>::from((
            limit_per_second,
            time_provider,
            SharedTracingSleepUntil::new(),
        ));

        let rate_limiter_cloned = rate_limiter.share();

        let (rate_limiter, deadline) = RateLimiter::rate_limit(rate_limiter, 5).await;
        assert_eq!(deadline, Some(now + Duration::from_millis(500)));
        let (_, deadline) = RateLimiter::rate_limit(rate_limiter_cloned, 5).await;
        assert_eq!(deadline, None,);

        *time_to_return.write() = now + Duration::from_millis(1500);

        let (_, deadline) = RateLimiter::rate_limit(rate_limiter, 10).await;
        assert_eq!(deadline, None);
    }

    #[tokio::test]
    async fn peers_receive_at_least_one_token_per_second() {
        let limit_per_second = 1.try_into().expect("1 > 0 qed");
        let now = Instant::now();
        let time_to_return = Arc::new(parking_lot::RwLock::new(now));
        let time_provider = time_to_return.clone();
        let time_provider: Arc<Box<dyn TimeProvider + Send + Sync>> =
            Arc::new(Box::new(move || *time_provider.read()));

        let rate_limiter = TracingRateLimiter::<_>::from((
            limit_per_second,
            time_provider,
            SharedTracingSleepUntil::new(),
        ));

        *time_to_return.write() = now + Duration::from_secs(1);

        let rate_limiter_cloned = rate_limiter.share();

        let (rate_limiter, deadline) = RateLimiter::rate_limit(rate_limiter, 1).await;
        assert_eq!(deadline, None);

        let (rate_limiter_cloned, deadline) = RateLimiter::rate_limit(rate_limiter_cloned, 1).await;
        assert_eq!(deadline, None);

        *time_to_return.write() = now + Duration::from_secs(2);

        let (_, deadline) = RateLimiter::rate_limit(rate_limiter, 1).await;
        assert_eq!(deadline, None);
        let (_, deadline) = RateLimiter::rate_limit(rate_limiter_cloned, 2).await;
        assert_eq!(deadline, Some(now + Duration::from_secs(3)));
    }

    /// Synchronizes all instances of [TokenBucket] using [tokio::sync::Barrier]. It should allow all of such instances to
    /// recognize presence of all the other peers that also allocated some bandwidth and then recalculate their own.
    struct SleepUntilWithBarrier<SU> {
        wrapped: SU,
        barrier: Arc<tokio::sync::RwLock<tokio::sync::Barrier>>,
        initial_counter: u64,
        counter: u64,
        // this is to overcome lack of `Cancellation Safety` of the method [Barrier::wait()].
        // Implementations of [SleepUntil::sleep_until()] should be cancellation safe.
        to_wait: Option<BoxFuture<'static, ()>>,
        id: u64,
    }

    impl<SU> Clone for SleepUntilWithBarrier<SU>
    where
        SU: Clone,
    {
        fn clone(&self) -> Self {
            Self {
                wrapped: self.wrapped.clone(),
                barrier: self.barrier.clone(),
                initial_counter: self.initial_counter,
                counter: self.counter,
                to_wait: None,
                id: self.id + 1,
            }
        }
    }

    impl<SU> SleepUntilWithBarrier<SU> {
        pub fn new(
            sleep_until: SU,
            barrier: Arc<tokio::sync::RwLock<tokio::sync::Barrier>>,
            how_many_times_to_use_barrier: u64,
        ) -> Self {
            Self {
                wrapped: sleep_until,
                barrier,
                initial_counter: how_many_times_to_use_barrier,
                counter: how_many_times_to_use_barrier,
                to_wait: None,
                id: 0,
            }
        }

        pub fn reset(&mut self) {
            self.counter = self.initial_counter;
            self.to_wait = None;
        }

        pub async fn wait(&mut self) {
            while self.counter > 0 {
                self.to_wait
                    .get_or_insert_with(|| {
                        let barrier = self.barrier.clone();
                        Box::pin(async move {
                            barrier.read().await.wait().await;
                        })
                    })
                    .await;
                self.to_wait = None;
                self.counter -= 1;
            }
        }
    }

    impl<SU> SleepUntil for SleepUntilWithBarrier<SU>
    where
        SU: SleepUntil + Send,
    {
        async fn sleep_until(&mut self, instant: Instant) {
            self.wait().await;
            self.wrapped.sleep_until(instant).await;
        }
    }

    #[tokio::test]
    async fn avarage_bandwidth_should_be_within_some_reasonable_bounds() {
        use rand::{
            distributions::{Distribution, Uniform},
            seq::SliceRandom,
            SeedableRng,
        };

        let mut test_state = Vec::new();

        let rate_limit = 4 * 1024 * 1024;
        let limit_per_second = rate_limit.try_into().expect("(4 * 1024 * 1024) > 0 qed");

        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("back to the future")
            .as_secs();
        let mut rand_generator = rand::rngs::StdRng::seed_from_u64(seed);

        let data_generator = Uniform::from((rate_limit + 1)..100 * rate_limit);
        let limiters_count = Uniform::from(10..=128).sample(&mut rand_generator);
        let batch_generator = Uniform::from(2..=limiters_count);

        let initial_time = Instant::now();
        let time_to_return = Arc::new(parking_lot::RwLock::new(initial_time));
        let time_provider = time_to_return.clone();
        let time_provider: Arc<Box<dyn TimeProvider + Send + Sync>> =
            Arc::new(Box::new(move || *time_provider.read()));

        let test_sleep_until_shared = SharedTracingSleepUntil::new();
        let last_deadline = test_sleep_until_shared.last_deadline.clone();

        let barrier = Arc::new(tokio::sync::RwLock::new(tokio::sync::Barrier::new(0)));
        let how_many_times_should_stop_on_barrier = 2;
        let test_sleep_until_with_barrier = SleepUntilWithBarrier::new(
            test_sleep_until_shared,
            barrier.clone(),
            how_many_times_should_stop_on_barrier,
        );
        let sleep_until_after_time_change =
            SleepUntilAfterChange::new(test_sleep_until_with_barrier);
        let rate_limiter = SharedTokenBucket::<_, _>::from((
            limit_per_second,
            time_provider,
            sleep_until_after_time_change,
        ));

        let mut rate_limiters: Vec<_> = repeat(())
            .scan((0usize, rate_limiter), |(id, rate_limiter), _| {
                let new_rate_limiter = rate_limiter.share();
                let new_state = rate_limiter.share();
                let limiter_id = *id;
                *rate_limiter = new_state;
                *id += 1;
                Some((limiter_id, Some(new_rate_limiter)))
            })
            .take(limiters_count)
            .collect();

        let mut total_data_scheduled = 0;
        let mut total_number_of_calls = 0;
        while total_number_of_calls < 1000 {
            let batch_size = batch_generator.sample(&mut rand_generator);

            total_number_of_calls += batch_size;
            *barrier.write().await = tokio::sync::Barrier::new(batch_size);

            rate_limiters.shuffle(&mut rand_generator);

            let mut batch_data = 0;
            let start_time = *time_to_return.read();
            let mut batch_state = Vec::new();
            let mut batch_test: FuturesOrdered<_> = rate_limiters[0..batch_size]
                .iter_mut()
                .zip((0..batch_size).rev())
                .map(|((selected_limiter_id, selected_rate_limiter), idx)| {
                    let data_read = data_generator.sample(&mut rand_generator);

                    let mut rate_limiter = selected_rate_limiter
                        .take()
                        .expect("we should be able to retrieve a rate-limiter");

                    // last instance won't be notified - its bandwidth will not change until some other instance finishes
                    rate_limiter
                        .rate_limiter
                        .sleep_until
                        .set_number_of_changes_to_wait(min(1, idx));
                    rate_limiter.rate_limiter.sleep_until.wrapped.reset();

                    let rate_task = SharedTokenBucket::rate_limit(rate_limiter, data_read);

                    batch_state.push((*selected_limiter_id, data_read));

                    total_data_scheduled += u128::from(data_read);
                    batch_data += data_read;

                    async move {
                        let rate_limiter = rate_task.await;

                        (rate_limiter, selected_rate_limiter)
                    }
                })
                .collect();

            test_state.push(batch_state);

            while let Some((rate_limiter, store)) = batch_test.next().await {
                *store = Some(rate_limiter);
            }

            let current_time = max(
                *time_to_return.read(),
                (*last_deadline.lock()).unwrap_or(*time_to_return.read()),
            );

            let batch_time: u64 = max((current_time - start_time).as_millis(), 1000)
                .try_into()
                .expect("something wrong with our time calculations");
            let rate = batch_data * 1000 / batch_time;
            let abs_rate_diff = rate.abs_diff(rate_limit);
            // in worst case, utilized bandwidth should be equal to `bandwidth * (1 + 1/2 + ... + 1/n) ≈ bandwidth * (ln n + O(1))`
            let max_possible_bandwidth =
                rate_limit as f64 * ((batch_size as f64).ln() + 1_f64).trunc();
            assert!(
                abs_rate_diff <= max_possible_bandwidth as u64,
                "Used bandwidth should be oscillating close to {rate_limit} b/s (+/- 50%), but got {rate} b/s instead. Total data sent: {total_data_scheduled}; Test data: {test_state:?}"
            );

            *time_to_return.write() = current_time;
        }
    }
}
