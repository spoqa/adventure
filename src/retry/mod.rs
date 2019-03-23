mod error;
#[cfg(feature = "backoff-tokio")]
mod tokio;
mod util;

use std::marker::PhantomData;
use std::ops::Deref;
use std::pin::Pin;
use std::time::Duration;

use pin_utils::unsafe_pinned;

#[cfg(feature = "backoff-tokio")]
pub use self::util::RetryBackoff;

use crate::repeat::RepeatableRequest;
use crate::request::Request;
use crate::response::Response;
use crate::task::{Poll, Waker};

pub use self::{
    error::{BackoffError, RetryError},
    util::Retry,
};
pub use backoff::{backoff::Backoff, ExponentialBackoff};

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

pub struct WithBackoff<R, T, C> {
    inner: R,
    _phantom: PhantomData<(T, C)>,
}

impl<R, T, C> WithBackoff<R, T, C> {
    pub(crate) fn new(req: R) -> Self {
        WithBackoff {
            inner: req,
            _phantom: PhantomData,
        }
    }
}

impl<R, T, C> Clone for WithBackoff<R, T, C>
where
    R: Clone,
{
    fn clone(&self) -> Self {
        WithBackoff {
            inner: self.inner.clone(),
            _phantom: PhantomData,
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.inner = source.inner.clone();
    }
}

impl<R, T, C> Unpin for WithBackoff<R, T, C> where R: Unpin {}

impl<R, T, C> Request<C> for WithBackoff<R, T, C>
where
    R: RetriableRequest<C>,
    T: Retry + Unpin,
    C: Clone,
{
    type Ok = R::Ok;
    type Error = RetryError<R::Error>;
    type Response = RetriableResponse<R, T, C>;

    fn into_response(self, client: C) -> Self::Response {
        RetriableResponse {
            client,
            request: self.inner,
            retry: T::new(),
            next: None,
            wait: None,
        }
    }
}

impl<R, T, C> RepeatableRequest<C> for WithBackoff<R, T, C>
where
    R: RetriableRequest<C> + Clone,
    T: Retry + Unpin,
    C: Clone,
{
    fn send(&self, client: C) -> Self::Response {
        self.clone().into_response(client)
    }
}

pub struct RetriableResponse<R, T, C>
where
    R: RetriableRequest<C>,
    T: Retry,
{
    client: C,
    request: R,
    retry: T,
    next: Option<R::Response>,
    wait: Option<T::Wait>,
}

impl<R, T, C> RetriableResponse<R, T, C>
where
    R: RetriableRequest<C>,
    T: Retry,
{
    unsafe_pinned!(request: R);
    unsafe_pinned!(retry: T);
    unsafe_pinned!(next: Option<R::Response>);
    unsafe_pinned!(wait: Option<T::Wait>);
}

impl<R, T, C> Unpin for RetriableResponse<R, T, C>
where
    R: RetriableRequest<C> + Unpin,
    R::Response: Unpin,
    T: Retry + Unpin,
    T::Wait: Unpin,
    C: Unpin,
{
}

impl<R, T, C> Response for RetriableResponse<R, T, C>
where
    R: RetriableRequest<C>,
    T: Retry + Unpin,
    C: Clone,
{
    type Ok = R::Ok;
    type Error = RetryError<R::Error>;

    fn poll(mut self: Pin<&mut Self>, waker: &Waker) -> Poll<Result<Self::Ok, Self::Error>> {
        if let Some(w) = self.as_mut().wait().as_pin_mut() {
            match w.poll(waker) {
                Poll::Pending => {
                    return Poll::Pending;
                }
                Poll::Ready(Err(e)) => {
                    return Poll::Ready(Err(From::from(e)));
                }
                _ => {}
            }
            self.as_mut().wait().set(None);
        }

        if self.as_mut().next().as_pin_mut().is_none() {
            let request = &self.as_ref().request;
            let next = request.send(self.client.clone());
            self.as_mut().next().set(Some(next));
        }

        match self
            .as_mut()
            .next()
            .as_pin_mut()
            .expect("Assertion failed")
            .poll(waker)
        {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(resp)) => Poll::Ready(Ok(resp)),
            Poll::Ready(Err(e)) => {
                self.as_mut().next().set(None);
                match self.as_mut().next_wait(e) {
                    Ok(w) => {
                        self.as_mut().wait().set(Some(w));
                        self.poll(waker)
                    }
                    Err(e) => Poll::Ready(Err(e)),
                }
            }
        }
    }
}

impl<R, T, C> RetriableResponse<R, T, C>
where
    R: RetriableRequest<C>,
    T: Retry + Unpin,
{
    fn next_wait(mut self: Pin<&mut Self>, err: R::Error) -> Result<T::Wait, RetryError<R::Error>> {
        let next = self.as_mut().retry().next_backoff()?;
        if self.as_ref().request.should_retry(&err, next) {
            Ok(self.as_ref().retry.wait(next))
        } else {
            Err(RetryError::from_err(err))
        }
    }
}
