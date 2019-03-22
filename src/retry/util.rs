use std::pin::Pin;

use super::{Backoff, BackoffError, RetryError};
use crate::compat::{Poll, Waker};
use crate::request::RetriableRequest;
use crate::response::Response;

pub trait Retry {
    type Wait: Response<Ok = (), Error = BackoffError>;

    fn wait(
        &mut self,
    ) -> Result<Self::Wait, BackoffError>;
}

pub(super) struct Waiting<W> {
    inner: WaitingImpl<W>,
}

enum WaitingImpl<W> {
    Wait(W),
    Timeout,
}

impl<R> From<Option<R>> for Waiting<R> {
    fn from(r: Option<R>) -> Self {
        let inner = r.map(WaitingImpl::Wait).unwrap_or(WaitingImpl::Timeout);
        Waiting { inner }
    }
}

impl<R> Response for Waiting<R>
where
    R: Response<Ok = (), Error = BackoffError> + Unpin,
{
    type Ok = ();
    type Error = BackoffError;

    fn poll(mut self: Pin<&mut Self>, w: &Waker) -> Poll<Result<Self::Ok, Self::Error>> {
        match &mut self.inner {
            WaitingImpl::Wait(fut) => Response::poll(Pin::new(fut), w),
            WaitingImpl::Timeout => Poll::Ready(Err(BackoffError::timeout())),
        }
    }
}
