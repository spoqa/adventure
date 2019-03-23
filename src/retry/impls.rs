use std::marker::PhantomData;
use std::pin::Pin;
use std::time::Duration;

use pin_utils::unsafe_pinned;

use super::{error::RetryError, RetriableRequest, RetriableResponse, Retry, Retrying};
use crate::repeat::RepeatableRequest;
use crate::request::{BaseRequest, Request};
use crate::response::Response;
use crate::task::{Poll, Waker};

#[doc(hidden)]
pub trait FnRetry<C> {
    type Error;
    fn call(&mut self, err: &Self::Error, next_interval: Duration) -> bool;
}

impl<R, F, C> FnRetry<C> for (R, F)
where
    F: FnMut(&R, &R::Error, Duration) -> bool,
    R: RepeatableRequest<C>,
{
    type Error = R::Error;
    fn call(&mut self, err: &Self::Error, next_interval: Duration) -> bool {
        (self.1)(&self.0, err, next_interval)
    }
}

impl<R, C> FnRetry<C> for (R, ())
where
    R: RetriableRequest,
{
    type Error = R::Error;
    fn call(&mut self, err: &Self::Error, next_interval: Duration) -> bool {
        self.0.should_retry(err, next_interval)
    }
}

impl<R, T> Retrying<R, T>
where
    R: RetriableRequest,
{
    pub(crate) fn new(req: R) -> Self {
        Retrying {
            inner: req,
            pred: (),
            _phantom: PhantomData,
        }
    }
}

impl<R, T, F> Retrying<R, T, F>
where
    T: Retry + Unpin,
{
    pub(crate) fn with_predicate(req: R, pred: F) -> Self {
        Retrying {
            inner: req,
            pred,
            _phantom: PhantomData,
        }
    }
}

impl<R, T, F> Clone for Retrying<R, T, F>
where
    R: Clone,
    F: Clone,
{
    fn clone(&self) -> Self {
        Retrying {
            inner: self.inner.clone(),
            pred: self.pred.clone(),
            _phantom: PhantomData,
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.inner = source.inner.clone();
        self.pred = source.pred.clone();
    }
}

impl<R, T, F> BaseRequest for Retrying<R, T, F>
where
    R: BaseRequest,
{
    type Ok = R::Ok;
    type Error = RetryError<R::Error>;
}

impl<R, T, F, C> Request<C> for Retrying<R, T, F>
where
    R: RepeatableRequest<C>,
    T: Retry + Unpin,
    (R, F): FnRetry<C, Error = R::Error> + Unpin,
    C: Clone,
{
    type Response = RetriableResponse<R, T, F, C>;

    fn into_response(self, client: C) -> Self::Response {
        RetriableResponse {
            client,
            request: (self.inner, self.pred),
            retry: T::new(),
            next: None,
            wait: None,
        }
    }
}

impl<R, T, F> Unpin for Retrying<R, T, F>
where
    R: Unpin,
    F: Unpin,
{
}

impl<R, T, F, C> Unpin for RetriableResponse<R, T, F, C>
where
    R: RepeatableRequest<C> + Unpin,
    R::Response: Unpin,
    T: Retry + Unpin,
    T::Wait: Unpin,
    F: Unpin,
    C: Unpin,
{
}

impl<R, T, F, C> Response for RetriableResponse<R, T, F, C>
where
    R: RepeatableRequest<C>,
    T: Retry + Unpin,
    (R, F): FnRetry<C, Error = R::Error> + Unpin,
    C: Clone,
{
    type Ok = R::Ok;
    type Error = RetryError<R::Error>;

    fn poll(self: Pin<&mut Self>, waker: &Waker) -> Poll<Result<Self::Ok, Self::Error>> {
        self.poll_impl(waker)
    }
}

impl<R, T, F, C> RetriableResponse<R, T, F, C>
where
    R: RepeatableRequest<C>,
    T: Retry + Unpin,
    (R, F): FnRetry<C, Error = R::Error> + Unpin,
    C: Clone,
{
    unsafe_pinned!(request: (R, F));
    unsafe_pinned!(retry: T);
    unsafe_pinned!(next: Option<R::Response>);
    unsafe_pinned!(wait: Option<T::Wait>);

    fn poll_impl(
        mut self: Pin<&mut Self>,
        waker: &Waker,
    ) -> Poll<Result<R::Ok, RetryError<R::Error>>> {
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
            let next = request.0.send(self.client.clone());
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
                        self.poll_impl(waker)
                    }
                    Err(e) => Poll::Ready(Err(e)),
                }
            }
        }
    }

    fn next_wait(mut self: Pin<&mut Self>, err: R::Error) -> Result<T::Wait, RetryError<R::Error>> {
        let next = self.as_mut().retry().next_backoff()?;
        if self.as_mut().request().get_mut().call(&err, next) {
            Ok(self.as_ref().retry.wait(next))
        } else {
            Err(RetryError::from_err(err))
        }
    }
}
