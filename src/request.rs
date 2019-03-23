//! A various trait of requests by its capabilities.
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::time::Duration;

use crate::response::Response;

/// A generalized request-response interface, regardless how client works.
pub trait Request<C> {
    /// The type of successful values from the corresponding response.
    type Ok;
    /// The type of failures from the corresponding response.
    type Error;
    /// The type of corresponding responses of this request.
    type Response: Response<Ok = Self::Ok, Error = Self::Error>;

    /// Send this request using the given client.
    fn into_response(self, client: C) -> Self::Response;
}

impl<R, C> Request<C> for Box<R>
where
    R: Request<C>,
{
    type Ok = R::Ok;
    type Error = R::Error;
    type Response = R::Response;
    fn into_response(self, client: C) -> Self::Response {
        let inner = *self;
        inner.into_response(client)
    }
}

pub trait RepeatableRequest<C>: Request<C> {
    fn send(&self, client: C) -> Self::Response;
}

impl<R, C> Request<C> for &R
where
    R: RepeatableRequest<C>,
{
    type Ok = R::Ok;
    type Error = R::Error;
    type Response = R::Response;
    fn into_response(self, client: C) -> Self::Response {
        self.send(client)
    }
}

impl<R, C> RepeatableRequest<C> for &R
where
    R: RepeatableRequest<C>,
{
    fn send(&self, client: C) -> Self::Response {
        (*self).send(client)
    }
}

impl<R, C> RepeatableRequest<C> for Box<R>
where
    R: RepeatableRequest<C>,
{
    fn send(&self, client: C) -> Self::Response {
        (**self).send(client)
    }
}

impl<P, C> Request<C> for Pin<P>
where
    P: Deref,
    <P as Deref>::Target: RepeatableRequest<C>,
{
    type Ok = <<P as Deref>::Target as Request<C>>::Ok;
    type Error = <<P as Deref>::Target as Request<C>>::Error;
    type Response = <<P as Deref>::Target as Request<C>>::Response;
    fn into_response(self, client: C) -> Self::Response {
        self.send(client)
    }
}

impl<P, C> RepeatableRequest<C> for Pin<P>
where
    P: Deref,
    <P as Deref>::Target: RepeatableRequest<C>,
{
    fn send(&self, client: C) -> Self::Response {
        <<P as Deref>::Target>::send(self, client)
    }
}

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

/// A request able to send subsequent requests to enumerate the entire result.
pub trait PagedRequest<C>: RepeatableRequest<C> {
    /// Modify itself to retrive the next response, of return `false` if the
    /// given response was the last one.
    fn advance(&mut self, response: &Self::Ok) -> bool;
}

impl<R, C> PagedRequest<C> for Box<R>
where
    R: PagedRequest<C>,
{
    fn advance(&mut self, response: &Self::Ok) -> bool {
        (**self).advance(response)
    }
}

impl<P, C> PagedRequest<C> for Pin<P>
where
    P: DerefMut,
    <P as Deref>::Target: PagedRequest<C> + Unpin,
{
    fn advance(&mut self, response: &Self::Ok) -> bool {
        self.as_mut().get_mut().advance(response)
    }
}
