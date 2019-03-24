use std::pin::Pin;
use std::time::Duration;

use pin_utils::unsafe_pinned;

use super::{error::RetryError, Backoff, ExponentialBackoff, RetriableRequest, Timer};
use crate::oneshot::OneshotRequest;
use crate::request::{BaseRequest, Request};
use crate::response::Response;
use crate::task::{Poll, Waker};

#[derive(Clone)]
pub struct Retrying<R, T, F = (), B = ExponentialBackoff> {
    inner: R,
    pred: F,
    backoff: B,
    timer: T,
}

impl<R, T, B> Retrying<R, T, (), B>
where
    R: BaseRequest,
    T: Timer + Default + Unpin,
    B: Backoff + Default,
{
    pub(crate) fn from_default(req: R) -> Self {
        Self::new(req, T::default(), B::default())
    }
}

impl<R, T, B> Retrying<R, T, (), B>
where
    R: BaseRequest,
    T: Timer + Unpin,
    B: Backoff,
{
    pub(crate) fn new(req: R, timer: T, backoff: B) -> Self {
        Retrying {
            inner: req,
            pred: (),
            backoff,
            timer,
        }
    }

    pub(crate) fn with_predicate<F>(self, pred: F) -> Retrying<R, T, F, B>
    where
        F: Fn(&R, &<R as BaseRequest>::Error, Duration) -> bool,
    {
        Retrying {
            inner: self.inner,
            pred,
            backoff: self.backoff,
            timer: self.timer,
        }
    }
}

impl<R, T, F, B> BaseRequest for Retrying<R, T, F, B>
where
    R: BaseRequest,
{
    type Ok = R::Ok;
    type Error = RetryError<R::Error>;
}

impl<R, T, F, B, C> OneshotRequest<C> for Retrying<R, T, F, B>
where
    Self: RetryMethod<C, Response = R::Response> + Unpin,
    R: Request<C>,
    R::Response: Unpin,
    C: Clone,
{
    type Response = Retrial<Self, C>;

    fn send_once(self, client: C) -> Self::Response {
        Retrial {
            client,
            request: self,
            next: None,
            wait: None,
        }
    }
}

impl<R, T, F, B> Unpin for Retrying<R, T, F, B>
where
    R: Unpin,
    F: Unpin,
    B: Unpin,
{
}

type WaitError<T, C> = <<T as RetryMethod<C>>::Response as Response>::Error;
type WaitResult<T, C> = Result<<T as RetryMethod<C>>::Delay, RetryError<WaitError<T, C>>>;

#[doc(hidden)]
pub trait RetryMethod<C> {
    type Response: Response;
    type Delay: Response<Ok = (), Error = RetryError>;

    fn send(&self, client: C) -> Self::Response;
    fn next_backoff(&mut self) -> Option<Duration>;
    fn check_retry(&mut self, err: &WaitError<Self, C>, next_duration: Duration) -> bool;

    fn expires_in(&mut self, next_duration: Duration) -> Self::Delay;

    fn next_wait(&mut self, err: WaitError<Self, C>) -> WaitResult<Self, C> {
        let next = self.next_backoff().ok_or_else(RetryError::timeout)?;
        if self.check_retry(&err, next) {
            Ok(self.expires_in(next))
        } else {
            Err(RetryError::from_err(err))
        }
    }
}

impl<R, T, B, C> RetryMethod<C> for Retrying<R, T, (), B>
where
    R: Request<C> + RetriableRequest,
    T: Timer,
    B: Backoff,
{
    type Response = R::Response;
    type Delay = T::Delay;

    fn send(&self, client: C) -> Self::Response {
        self.inner.send(client)
    }

    fn next_backoff(&mut self) -> Option<Duration> {
        self.backoff.next_backoff()
    }

    fn check_retry(
        &mut self,
        err: &<Self::Response as Response>::Error,
        next_interval: Duration,
    ) -> bool {
        self.inner.should_retry(err, next_interval)
    }

    fn expires_in(&mut self, next_duration: Duration) -> Self::Delay {
        self.timer.expires_in(next_duration)
    }
}

impl<R, T, F, B, C> RetryMethod<C> for Retrying<R, T, F, B>
where
    R: Request<C>,
    T: Timer,
    F: FnMut(&R, &R::Error, Duration) -> bool,
    B: Backoff,
{
    type Response = R::Response;
    type Delay = T::Delay;

    fn send(&self, client: C) -> Self::Response {
        self.inner.send(client)
    }

    fn next_backoff(&mut self) -> Option<Duration> {
        self.backoff.next_backoff()
    }

    fn check_retry(
        &mut self,
        err: &<Self::Response as Response>::Error,
        next_interval: Duration,
    ) -> bool {
        (self.pred)(&self.inner, err, next_interval)
    }

    fn expires_in(&mut self, next_duration: Duration) -> Self::Delay {
        self.timer.expires_in(next_duration)
    }
}

/// Response for [`retry`](crate::util::RequestExt::retry) combinator.
pub struct Retrial<R, C>
where
    R: RetryMethod<C>,
{
    client: C,
    request: R,
    next: Option<R::Response>,
    wait: Option<R::Delay>,
}

impl<R, C> Unpin for Retrial<R, C>
where
    R: RetryMethod<C> + Unpin,
    R::Response: Unpin,
    C: Unpin,
{
}

impl<R, C> Response for Retrial<R, C>
where
    R: RetryMethod<C> + Unpin,
    R::Response: Unpin,
    C: Clone,
{
    type Ok = <R::Response as Response>::Ok;
    type Error = RetryError<WaitError<R, C>>;

    fn poll(self: Pin<&mut Self>, waker: &Waker) -> Poll<Result<Self::Ok, Self::Error>> {
        self.poll_impl(waker)
    }
}

impl<R, C> Retrial<R, C>
where
    R: RetryMethod<C> + Unpin,
    R::Response: Unpin,
    C: Clone,
{
    unsafe_pinned!(request: R);
    unsafe_pinned!(next: Option<R::Response>);
    unsafe_pinned!(wait: Option<R::Delay>);

    fn poll_impl(
        mut self: Pin<&mut Self>,
        waker: &Waker,
    ) -> Poll<Result<<R::Response as Response>::Ok, RetryError<WaitError<R, C>>>> {
        if let Some(w) = self.as_mut().wait().as_pin_mut() {
            match w.poll(waker) {
                Poll::Pending => {
                    return Poll::Pending;
                }
                Poll::Ready(Err(e)) => {
                    return Poll::Ready(Err(e.transform()));
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
                match self.as_mut().request().get_mut().next_wait(e) {
                    Ok(w) => {
                        self.as_mut().wait().set(Some(w));
                        self.poll_impl(waker)
                    }
                    Err(e) => Poll::Ready(Err(e)),
                }
            }
        }
    }
}
