mod error;
mod impls;
#[cfg(feature = "backoff-tokio")]
mod tokio;
mod util;

use std::marker::PhantomData;
use std::ops::Deref;
use std::pin::Pin;
use std::time::Duration;

#[cfg(feature = "backoff-tokio")]
pub use self::util::RetryBackoff;

use crate::repeat::RepeatableRequest;

pub use self::{
    error::{BackoffError, RetryError},
    util::Retry,
};
pub use backoff::{backoff::Backoff, ExponentialBackoff};

pub trait RetriableRequest<C>: RepeatableRequest<C> {
    fn should_retry(&self, error: &Self::Error, next_interval: Duration) -> bool;
}

impl<R, C> RetriableRequest<C> for &R
where
    R: RetriableRequest<C>,
{
    fn should_retry(&self, error: &Self::Error, next_interval: Duration) -> bool {
        (*self).should_retry(error, next_interval)
    }
}

impl<R, C> RetriableRequest<C> for Box<R>
where
    R: RetriableRequest<C>,
{
    fn should_retry(&self, error: &Self::Error, next_interval: Duration) -> bool {
        (**self).should_retry(error, next_interval)
    }
}

impl<P, C> RetriableRequest<C> for Pin<P>
where
    P: Deref,
    <P as Deref>::Target: RetriableRequest<C>,
{
    fn should_retry(&self, error: &Self::Error, next_interval: Duration) -> bool {
        <<P as Deref>::Target>::should_retry(self, error, next_interval)
    }
}

pub struct Retrying<R, T, F = ()> {
    inner: R,
    pred: F,
    _phantom: PhantomData<T>,
}

pub struct RetriableResponse<R, T, F, C>
where
    R: RepeatableRequest<C>,
    T: Retry,
{
    client: C,
    request: (R, F),
    retry: T,
    next: Option<R::Response>,
    wait: Option<T::Wait>,
}
