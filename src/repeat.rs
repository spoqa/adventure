use std::ops::Deref;
use std::pin::Pin;

use crate::request::Request;

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

#[derive(Clone)]
pub struct Repeat<R> {
    inner: R,
}

impl<R> From<R> for Repeat<R>
where
    R: Clone,
{
    fn from(req: R) -> Self {
        Repeat { inner: req }
    }
}

impl<R, C> Request<C> for Repeat<R>
where
    R: Request<C>,
{
    type Ok = R::Ok;
    type Error = R::Error;
    type Response = R::Response;

    fn into_response(self, client: C) -> Self::Response {
        self.inner.into_response(client)
    }
}

impl<R, C> RepeatableRequest<C> for Repeat<R>
where
    R: Request<C> + Clone,
{
    fn send(&self, client: C) -> Self::Response {
        self.clone().into_response(client)
    }
}
