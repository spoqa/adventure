use std::time::Duration;

#[cfg(feature = "backoff-tokio")]
use super::tokio::Delay;
use super::{Backoff, BackoffError, ExponentialBackoff};
use crate::response::Response;

pub trait Retry {
    type Wait: Response<Ok = (), Error = BackoffError>;

    fn new() -> Self;
    fn next_backoff(&mut self) -> Result<Duration, BackoffError>;
    fn wait(&self, interval: Duration) -> Self::Wait;
}

#[cfg(feature = "backoff-tokio")]
pub struct RetryBackoff {
    backoff: ExponentialBackoff,
}

#[cfg(feature = "backoff-tokio")]
impl Retry for RetryBackoff {
    type Wait = Delay;

    fn new() -> Self {
        RetryBackoff {
            backoff: ExponentialBackoff::default(),
        }
    }

    fn next_backoff(&mut self) -> Result<Duration, BackoffError> {
        self.backoff
            .next_backoff()
            .ok_or_else(BackoffError::timeout)
    }

    fn wait(&self, interval: Duration) -> Self::Wait {
        Delay::expires_in(interval)
    }
}
