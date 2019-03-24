use crate::oneshot::OneshotRequest;
use crate::request::{BaseRequest, Request};

/// An [`Request`] adaptor for types that implements [`OneshotRequest`], by
/// cloning itself.
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

impl<R, C> Request<C> for Repeat<R>
where
    R: OneshotRequest<C> + Clone,
{
    type Response = R::Response;

    fn send(&self, client: C) -> Self::Response {
        self.inner.clone().send_once(client)
    }
}
