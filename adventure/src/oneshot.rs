//! A base trait represents a request.
use crate::request::{BaseRequest, RepeatableRequest};
use crate::response::Response;

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
