pub mod backoff;
#[cfg(feature = "tokio-timer")]
pub mod tokio;

mod error;
mod impls;

use core::ops::Deref;
use core::pin::Pin;
use core::time::Duration;

use crate::request::BaseRequest;
use crate::response::Response;

#[cfg(feature = "tokio-timer")]
#[doc(inline)]
pub use self::tokio::TokioTimer;
pub use self::{
    backoff::{Backoff, ExponentialBackoff},
    error::RetryError,
    impls::{Retrial, RetrialPredicate, Retrying},
};

#[cfg(feature = "tokio-timer")]
pub type RetryingTokio<R, B = ExponentialBackoff, F = ()> = Retrying<R, TokioTimer, B, F>;

/// A request able to decide to send itself again if the previous attempt has failed.
pub trait RetriableRequest: BaseRequest {
    fn should_retry(&self, error: &Self::Error, next_interval: Duration) -> bool;

    /// Wrap this request to retry itself on failure, with a default [`ExponentialBackoff`] strategy.
    ///
    /// It should be called within the tokio execution context,
    /// because the default timer is implemented using [`tokio_timer`].
    #[cfg(feature = "tokio-timer")]
    fn retry(self) -> RetryingTokio<Self>
    where
        Self: Sized,
    {
        RetryingTokio::from_default(self)
    }

    /// Wrap this request to retry itself on failure, with a given backoff strategy.
    ///
    /// It should be called within the tokio execution context,
    /// because the default timer is implemented using [`tokio_timer`].
    #[cfg(feature = "tokio-timer")]
    fn retry_with_backoff<B>(self, backoff: B) -> RetryingTokio<Self, B>
    where
        Self: BaseRequest + Sized,
        B: Backoff,
    {
        RetryingTokio::new(self, Default::default(), backoff)
    }
}

impl<R> RetriableRequest for &R
where
    R: RetriableRequest,
{
    fn should_retry(&self, error: &Self::Error, next_interval: Duration) -> bool {
        (*self).should_retry(error, next_interval)
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

#[cfg(feature = "alloc")]
mod feature_alloc {
    use alloc::boxed::Box;

    use super::*;

    impl<R> RetriableRequest for Box<R>
    where
        R: RetriableRequest,
    {
        fn should_retry(&self, error: &Self::Error, next_interval: Duration) -> bool {
            (**self).should_retry(error, next_interval)
        }
    }
}
