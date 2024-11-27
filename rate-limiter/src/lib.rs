mod rate_limiter;
mod token_bucket;

use std::num::{NonZeroU64, TryFromIntError};

use futures::{future::BoxFuture, ready, FutureExt};
use rate_limiter::RateLimiterFacade;
use tokio::io::AsyncRead;

pub use crate::{rate_limiter::SharedRateLimiter, token_bucket::SharedTokenBucket};

const LOG_TARGET: &str = "rate-limiter";

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct NonZeroRatePerSecond(NonZeroU64);

pub const MIN: NonZeroRatePerSecond = NonZeroRatePerSecond(NonZeroU64::MIN);

impl From<NonZeroRatePerSecond> for NonZeroU64 {
    fn from(NonZeroRatePerSecond(value): NonZeroRatePerSecond) -> Self {
        value
    }
}

impl From<NonZeroRatePerSecond> for u64 {
    fn from(NonZeroRatePerSecond(value): NonZeroRatePerSecond) -> Self {
        value.into()
    }
}

impl From<NonZeroU64> for NonZeroRatePerSecond {
    fn from(value: NonZeroU64) -> Self {
        NonZeroRatePerSecond(value)
    }
}

impl TryFrom<u64> for NonZeroRatePerSecond {
    type Error = TryFromIntError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Ok(NonZeroRatePerSecond(value.try_into()?))
    }
}

#[derive(PartialEq, Eq)]
pub enum RatePerSecond {
    Block,
    Rate(NonZeroRatePerSecond),
}

impl From<RatePerSecond> for u64 {
    fn from(value: RatePerSecond) -> Self {
        match value {
            RatePerSecond::Block => 0,
            RatePerSecond::Rate(NonZeroRatePerSecond(value)) => value.into(),
        }
    }
}

impl From<u64> for RatePerSecond {
    fn from(value: u64) -> Self {
        NonZeroU64::try_from(value)
            .map(NonZeroRatePerSecond::from)
            .map(Self::Rate)
            .unwrap_or(Self::Block)
    }
}

impl From<NonZeroRatePerSecond> for RatePerSecond {
    fn from(value: NonZeroRatePerSecond) -> Self {
        RatePerSecond::Rate(value)
    }
}

pub struct RateLimitedAsyncRead<Read> {
    rate_limiter: BoxFuture<'static, RateLimiterFacade>,
    inner: Read,
}

impl<Read> RateLimitedAsyncRead<Read> {
    pub fn new(read: Read, rate_limiter: RateLimiterFacade) -> Self {
        Self {
            rate_limiter: Box::pin(rate_limiter.rate_limit(0)),
            inner: read,
        }
    }

    pub fn inner(&self) -> &Read {
        &self.inner
    }

    /// Helper method for the use of the [AsyncRead](tokio::io::AsyncRead) implementation.
    fn rate_limit(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>>
    where
        Read: AsyncRead + Unpin,
    {
        let this = self.get_mut();
        let read = std::pin::Pin::new(&mut this.inner);

        let sleeping_rate_limiter = ready!(this.rate_limiter.poll_unpin(cx));

        let filled_before = buf.filled().len();
        let result = read.poll_read(cx, buf);
        let filled_after: &[u8] = buf.filled();
        let filled_after = filled_after.len().saturating_mul(8);
        let last_read_size = filled_after.saturating_sub(filled_before);

        this.rate_limiter = sleeping_rate_limiter.rate_limit(last_read_size).boxed();

        result
    }
}

impl<Read> AsyncRead for RateLimitedAsyncRead<Read>
where
    Read: AsyncRead + Unpin,
{
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        self.rate_limit(cx, buf)
    }
}

pub struct FuturesRateLimitedAsyncReadWrite<ReadWrite> {
    rate_limiter: BoxFuture<'static, RateLimiterFacade>,
    inner: ReadWrite,
}

impl<ReadWrite> FuturesRateLimitedAsyncReadWrite<ReadWrite> {
    pub fn new(wrapped: ReadWrite, rate_limiter: RateLimiterFacade) -> Self {
        Self {
            rate_limiter: Box::pin(rate_limiter.rate_limit(0)),
            inner: wrapped,
        }
    }

    fn get_inner(self: std::pin::Pin<&mut Self>) -> std::pin::Pin<&mut ReadWrite>
    where
        ReadWrite: Unpin,
    {
        let this = self.get_mut();
        std::pin::Pin::new(&mut this.inner)
    }

    /// Helper method for the use of the [AsyncRead](futures::AsyncRead) implementation.
    fn rate_limit(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>>
    where
        ReadWrite: futures::AsyncRead + Unpin,
    {
        let this = self.get_mut();
        let read = std::pin::Pin::new(&mut this.inner);

        let sleeping_rate_limiter = ready!(this.rate_limiter.poll_unpin(cx));

        let result = read.poll_read(cx, buf);
        let last_read_size = match &result {
            std::task::Poll::Ready(Ok(read_size)) => 8 * *read_size,
            _ => 0,
        };

        this.rate_limiter = sleeping_rate_limiter.rate_limit(last_read_size).boxed();

        result
    }
}

impl<Read> futures::AsyncRead for FuturesRateLimitedAsyncReadWrite<Read>
where
    Read: futures::AsyncRead + Unpin,
{
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        self.rate_limit(cx, buf)
    }
}

impl<Write> futures::AsyncWrite for FuturesRateLimitedAsyncReadWrite<Write>
where
    Write: futures::AsyncWrite + Unpin,
{
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        self.get_inner().poll_write(cx, buf)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        self.get_inner().poll_flush(cx)
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        self.get_inner().poll_close(cx)
    }
}
