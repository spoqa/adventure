use crate::request::{BaseRequest, Request};
use crate::response::Response;

/// A request that can be sent just once.
pub trait OneshotRequest<C>: BaseRequest {
    /// The type of corresponding responses of this request.
    type Response: Response<Ok = Self::Ok, Error = Self::Error>;

    /// Send this request to the given client, by consuming itself.
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

impl<R> From<R> for Oneshot<R> {
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
    R: Request<C>,
{
    type Response = R::Response;

    fn send_once(self, client: C) -> Self::Response {
        self.inner.send(client)
    }
}
