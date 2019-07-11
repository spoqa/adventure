use core::pin::Pin;

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

    fn send(self: Pin<&mut Self>, client: C) -> Self::Response {
        self.inner.clone().send_once(client)
    }
}

#[cfg(feature = "backoff")]
mod impl_retry {
    use core::time::Duration;

    use super::Repeat;
    use crate::retry::RetriableRequest;

    impl<R> RetriableRequest for Repeat<R>
    where
        R: RetriableRequest,
    {
        fn should_retry(&self, error: &Self::Error, next_interval: Duration) -> bool {
            self.inner.should_retry(error, next_interval)
        }
    }
}

mod impl_paginator {
    use super::Repeat;
    use crate::paginator::PagedRequest;

    impl<R> PagedRequest for Repeat<R>
    where
        R: PagedRequest,
    {
        fn advance(&mut self, response: &Self::Ok) -> bool {
            self.inner.advance(response)
        }
    }
}
