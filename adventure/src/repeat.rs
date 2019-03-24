use std::ops::Deref;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

use crate::request::{BaseRequest, OneshotRequest};
use crate::response::Response;

pub trait RepeatableRequest<C>: BaseRequest {
    /// The type of corresponding responses of this request.
    type Response: Response<Ok = Self::Ok, Error = Self::Error>;

    fn send(&self, client: C) -> Self::Response;
}

impl<R, C> RepeatableRequest<C> for &R
where
    R: RepeatableRequest<C>,
{
    type Response = R::Response;
    fn send(&self, client: C) -> Self::Response {
        (*self).send(client)
    }
}

impl<R, C> RepeatableRequest<C> for Box<R>
where
    R: RepeatableRequest<C>,
{
    type Response = R::Response;
    fn send(&self, client: C) -> Self::Response {
        (**self).send(client)
    }
}

impl<R, C> RepeatableRequest<C> for Rc<R>
where
    R: RepeatableRequest<C>,
{
    type Response = R::Response;
    fn send(&self, client: C) -> Self::Response {
        (**self).send(client)
    }
}

impl<R, C> RepeatableRequest<C> for Arc<R>
where
    R: RepeatableRequest<C>,
{
    type Response = R::Response;
    fn send(&self, client: C) -> Self::Response {
        (**self).send(client)
    }
}

impl<P, C> RepeatableRequest<C> for Pin<P>
where
    P: Deref,
    <P as Deref>::Target: RepeatableRequest<C>,
{
    type Response = <<P as Deref>::Target as RepeatableRequest<C>>::Response;
    fn send(&self, client: C) -> Self::Response {
        (**self).send(client)
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

impl<R> BaseRequest for Repeat<R>
where
    R: BaseRequest,
{
    type Ok = R::Ok;
    type Error = R::Error;
}

impl<R, C> OneshotRequest<C> for Repeat<R>
where
    R: OneshotRequest<C> + Clone,
{
    type Response = R::Response;

    fn send_once(self, client: C) -> Self::Response {
        self.inner.send_once(client)
    }
}

impl<R, C> RepeatableRequest<C> for Repeat<R>
where
    R: OneshotRequest<C> + Clone,
{
    type Response = R::Response;

    fn send(&self, client: C) -> Self::Response {
        self.inner.clone().send_once(client)
    }
}

#[derive(Clone)]
pub struct Oneshot<R> {
    inner: R,
}

impl<R> From<R> for Oneshot<R>
where
    R: Clone,
{
    fn from(req: R) -> Self {
        Oneshot { inner: req }
    }
}

impl<R> BaseRequest for Oneshot<R>
where
    R: BaseRequest,
{
    type Ok = R::Ok;
    type Error = R::Error;
}

impl<R, C> OneshotRequest<C> for Oneshot<R>
where
    R: RepeatableRequest<C>,
{
    type Response = R::Response;

    fn send_once(self, client: C) -> Self::Response {
        self.inner.send(client)
    }
}
