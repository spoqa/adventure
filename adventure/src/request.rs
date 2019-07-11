//! A base trait represents a request.
use core::ops::{Deref, DerefMut};
use core::pin::Pin;

use crate::oneshot::Oneshot;
use crate::response::Response;

#[cfg(feature = "backoff")]
use crate::retry::{Backoff, RetrialPredicate, Retrying, Timer};
#[cfg(all(feature = "backoff", feature = "tokio-timer"))]
use crate::retry::{ExponentialBackoff, RetryingTokio};

/// Trait to represent types of the request, and their expected output and
/// error types.
pub trait BaseRequest {
    /// The type of successful values from the corresponding response.
    type Ok;
    /// The type of failures from the corresponding response.
    type Error;
}

impl<R> BaseRequest for &R
where
    R: BaseRequest,
{
    type Ok = R::Ok;
    type Error = R::Error;
}

impl<P> BaseRequest for Pin<P>
where
    P: Deref,
    P::Target: BaseRequest,
{
    type Ok = <P::Target as BaseRequest>::Ok;
    type Error = <P::Target as BaseRequest>::Error;
}

/// A generalized request-response interface, regardless how client works.
///
/// Because that the type of a client is parametrized, it can be implemented
/// to work with various kind of clients for the same type of the request.
pub trait Request<C>: BaseRequest {
    /// The type of corresponding responses of this request.
    type Response: Response<Ok = Self::Ok, Error = Self::Error>;

    fn send(self: Pin<&mut Self>, client: C) -> Self::Response;

    fn oneshot(self) -> Oneshot<Self>
    where
        Self: Sized,
    {
        Oneshot::from(self)
    }

    /// Wrap this request to retry if the given predicate returns `true`.
    ///
    /// It should be called within the tokio execution context,
    /// because the default timer is implemented using [`tokio_timer`].
    #[cfg(all(feature = "backoff", feature = "tokio-timer"))]
    fn retry_if<F>(self, pred: F) -> RetryingTokio<Self, ExponentialBackoff, F>
    where
        Self: Sized,
        F: RetrialPredicate<Self>,
    {
        RetryingTokio::from_default(self).with_predicate(pred)
    }

    /// Wrap this request to retry with customizable options, including the timer implementation.
    #[cfg(feature = "backoff")]
    fn retry_with_config<T, B, F>(self, timer: T, pred: F, backoff: B) -> Retrying<Self, T, B, F>
    where
        Self: Sized,
        T: Timer + Unpin,
        B: Backoff,
        F: RetrialPredicate<Self>,
    {
        Retrying::new(self, timer, backoff).with_predicate(pred)
    }
}

impl<P, C> Request<C> for Pin<P>
where
    P: DerefMut + Unpin,
    <P as Deref>::Target: Request<C>,
{
    type Response = <<P as Deref>::Target as Request<C>>::Response;
    fn send(self: Pin<&mut Self>, client: C) -> Self::Response {
        <<P as Deref>::Target>::send(self.get_mut().as_mut(), client)
    }
}

#[cfg(feature = "alloc")]
mod feature_alloc {
    use alloc::{boxed::Box, rc::Rc, sync::Arc};

    use super::*;

    impl<R> BaseRequest for Box<R>
    where
        R: BaseRequest,
    {
        type Ok = R::Ok;
        type Error = R::Error;
    }

    impl<R> BaseRequest for Rc<R>
    where
        R: BaseRequest,
    {
        type Ok = R::Ok;
        type Error = R::Error;
    }

    impl<R> BaseRequest for Arc<R>
    where
        R: BaseRequest,
    {
        type Ok = R::Ok;
        type Error = R::Error;
    }

    impl<R, C> Request<C> for Box<R>
    where
        R: Request<C>,
    {
        type Response = R::Response;
        fn send(self: Pin<&mut Self>, client: C) -> Self::Response {
            let pinned: Pin<&mut R> = unsafe { self.map_unchecked_mut(|b| b.as_mut()) };
            R::send(pinned, client)
        }
    }
}
