mod error;
mod impls;
#[cfg(feature = "tokio-timer")]
pub mod tokio;

use std::ops::Deref;
use std::pin::Pin;
use std::time::Duration;

use crate::request::BaseRequest;
use crate::response::Response;

#[cfg(feature = "tokio-timer")]
pub use self::tokio::TokioTimer;
pub use self::{
    error::RetryError,
    impls::{RetriableResponse, RetryMethod, Retrying},
};
pub use backoff::{backoff::Backoff, ExponentialBackoff};

pub trait RetriableRequest: BaseRequest {
    fn should_retry(&self, error: &Self::Error, next_interval: Duration) -> bool;
}

impl<R> RetriableRequest for &R
where
    R: RetriableRequest,
{
    fn should_retry(&self, error: &Self::Error, next_interval: Duration) -> bool {
        (*self).should_retry(error, next_interval)
    }
}

impl<R> RetriableRequest for Box<R>
where
    R: RetriableRequest,
{
    fn should_retry(&self, error: &Self::Error, next_interval: Duration) -> bool {
        (**self).should_retry(error, next_interval)
    }
}

impl<P> RetriableRequest for Pin<P>
where
    P: Deref,
    <P as Deref>::Target: RetriableRequest,
{
    fn should_retry(&self, error: &Self::Error, next_interval: Duration) -> bool {
        <<P as Deref>::Target>::should_retry(self, error, next_interval)
    }
}

pub trait Timer {
    type Delay: Response<Ok = (), Error = RetryError>;

    fn expires_in(&mut self, interval: Duration) -> Self::Delay;
}
