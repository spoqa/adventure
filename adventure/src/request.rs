use std::ops::Deref;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

use crate::response::Response;

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
