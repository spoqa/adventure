//! A base trait represents a request.
use std::ops::Deref;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

use crate::oneshot::Oneshot;
use crate::response::Response;

#[cfg(all(feature = "backoff", feature = "tokio-timer"))]
use crate::retry::TokioTimer;
#[cfg(feature = "backoff")]
use crate::retry::{Backoff, RetrialPredicate, Retrying, Timer};

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

    fn send(&self, client: C) -> Self::Response;

    fn oneshot(self) -> Oneshot<Self>
    where
        Self: Sized,
    {
        Oneshot::from(self)
    }

    #[cfg(all(feature = "backoff", feature = "tokio-timer"))]
    fn retry_if<F>(self, pred: F) -> Retrying<Self, TokioTimer, F>
    where
        Self: Sized,
        F: RetrialPredicate<Self>,
    {
        Retrying::from_default(self).with_predicate(pred)
    }

    #[cfg(feature = "backoff")]
    fn retry_with_config<T, F, B>(self, timer: T, pred: F, backoff: B) -> Retrying<Self, T, F, B>
    where
        Self: Sized,
        T: Timer + Unpin,
        F: RetrialPredicate<Self>,
        B: Backoff,
    {
        Retrying::new(self, timer, backoff).with_predicate(pred)
    }
}

impl<R, C> Request<C> for &R
where
    R: Request<C>,
{
    type Response = R::Response;
    fn send(&self, client: C) -> Self::Response {
        (*self).send(client)
    }
}

impl<R, C> Request<C> for Box<R>
where
    R: Request<C>,
{
    type Response = R::Response;
    fn send(&self, client: C) -> Self::Response {
        (**self).send(client)
    }
}

impl<R, C> Request<C> for Rc<R>
where
    R: Request<C>,
{
    type Response = R::Response;
    fn send(&self, client: C) -> Self::Response {
        (**self).send(client)
    }
}

impl<R, C> Request<C> for Arc<R>
where
    R: Request<C>,
{
    type Response = R::Response;
    fn send(&self, client: C) -> Self::Response {
        (**self).send(client)
    }
}

impl<P, C> Request<C> for Pin<P>
where
    P: Deref,
    <P as Deref>::Target: Request<C>,
{
    type Response = <<P as Deref>::Target as Request<C>>::Response;
    fn send(&self, client: C) -> Self::Response {
        (**self).send(client)
    }
}
