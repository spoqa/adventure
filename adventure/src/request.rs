//! A base trait represents a request.
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

/// A generalized request-response interface, regardless how client works.
pub trait OneshotRequest<C>: BaseRequest {
    /// The type of corresponding responses of this request.
    type Response: Response<Ok = Self::Ok, Error = Self::Error>;

    /// Send this request using the given client.
    fn send_once(self, client: C) -> Self::Response;
}

impl<R, C> OneshotRequest<C> for Box<R>
where
    R: OneshotRequest<C>,
{
    type Response = R::Response;
    fn send_once(self, client: C) -> Self::Response {
        let inner = *self;
        inner.send_once(client)
    }
}
