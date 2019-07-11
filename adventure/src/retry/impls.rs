use core::pin::Pin;
use core::task::{Context, Poll};
use core::time::Duration;

use pin_utils::unsafe_pinned;

use super::{error::RetryError, Backoff, ExponentialBackoff, RetriableRequest, Timer};
use crate::oneshot::OneshotRequest;
use crate::paginator::PagedRequest;
use crate::request::{BaseRequest, Request};
use crate::response::Response;

pub trait RetrialPredicate<R>
where
    R: BaseRequest,
{
    fn should_retry(
        &self,
        req: &R,
        err: &<R as BaseRequest>::Error,
        next_interval: Duration,
    ) -> bool;
}

impl<F, R> RetrialPredicate<R> for F
where
    R: BaseRequest,
    F: Fn(&R, &<R as BaseRequest>::Error, Duration) -> bool,
{
    fn should_retry(
        &self,
        req: &R,
        err: &<R as BaseRequest>::Error,
        next_interval: Duration,
    ) -> bool {
        (self)(req, err, next_interval)
    }
}

impl<R> RetrialPredicate<R> for ()
where
    R: RetriableRequest,
{
    fn should_retry(
        &self,
        req: &R,
        err: &<R as BaseRequest>::Error,
        next_interval: Duration,
    ) -> bool {
        req.should_retry(err, next_interval)
    }
}

/// Request for [`retry`](crate::util::RequestExt::retry) combinator.
#[derive(Clone)]
pub struct Retrying<R, T, B = ExponentialBackoff, F = ()> {
    inner: R,
    timer: T,
    backoff: B,
    pred: F,
}

impl<R, T, B> Retrying<R, T, B>
where
    R: BaseRequest,
    T: Timer + Default + Unpin,
    B: Backoff + Default,
{
    pub(crate) fn from_default(req: R) -> Self {
        Self::new(req, T::default(), B::default())
    }
}

impl<R, T, B> Retrying<R, T, B>
where
    R: BaseRequest,
    T: Timer + Unpin,
    B: Backoff,
{
    pub(crate) fn new(req: R, timer: T, backoff: B) -> Self {
        Retrying {
            inner: req,
            timer,
            backoff,
            pred: (),
        }
    }

    pub(crate) fn with_predicate<F>(self, pred: F) -> Retrying<R, T, B, F>
    where
        F: RetrialPredicate<R>,
    {
        Retrying {
            inner: self.inner,
            timer: self.timer,
            backoff: self.backoff,
            pred,
        }
    }
}

impl<R, T, B, F> Retrying<R, T, B, F>
where
    R: BaseRequest,
{
    unsafe_pinned!(inner: R);
}

impl<R, T, B, F> BaseRequest for Retrying<R, T, B, F>
where
    R: BaseRequest,
{
    type Ok = R::Ok;
    type Error = RetryError<R::Error>;
}

impl<R, T, B, F, C> OneshotRequest<C> for Retrying<R, T, B, F>
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

impl<R, T, B, F, C> Request<C> for Retrying<R, T, B, F>
where
    Self: RetryMethod<C, Response = R::Response> + Clone + Unpin,
    R: Request<C>,
    R::Response: Unpin,
    C: Clone,
{
    type Response = Retrial<Self, C>;

    fn send(self: Pin<&mut Self>, client: C) -> Self::Response {
        Retrial {
            client,
            request: self.clone(),
            next: None,
            wait: None,
        }
    }
}

impl<R, T, B, F> PagedRequest for Retrying<R, T, B, F>
where
    R: PagedRequest,
{
    fn advance(&mut self, response: &Self::Ok) -> bool {
        self.inner.advance(response)
    }
}

impl<R, T, B, F> Unpin for Retrying<R, T, B, F>
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

    fn send(self: Pin<&mut Self>, client: C) -> Self::Response;
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

impl<R, T, B, F, C> RetryMethod<C> for Retrying<R, T, B, F>
where
    R: Request<C>,
    T: Timer,
    B: Backoff,
    F: RetrialPredicate<R>,
{
    type Response = R::Response;
    type Delay = T::Delay;

    fn send(self: Pin<&mut Self>, client: C) -> Self::Response {
        self.inner().send(client)
    }

    fn next_backoff(&mut self) -> Option<Duration> {
        self.backoff.next_backoff()
    }

    fn check_retry(
        &mut self,
        err: &<Self::Response as Response>::Error,
        next_interval: Duration,
    ) -> bool {
        self.pred.should_retry(&self.inner, err, next_interval)
    }

    fn expires_in(&mut self, next_duration: Duration) -> Self::Delay {
        self.timer.expires_in(next_duration)
    }
}

/// Response for [`retry`](crate::util::RequestExt::retry) combinator.
#[must_use = "responses do nothing unless polled"]
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

    fn try_poll(
        self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
    ) -> Poll<Result<Self::Ok, Self::Error>> {
        self.poll_impl(ctx)
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
        ctx: &mut Context<'_>,
    ) -> Poll<Result<<R::Response as Response>::Ok, RetryError<WaitError<R, C>>>> {
        if let Some(w) = self.as_mut().wait().as_pin_mut() {
            match w.try_poll(ctx) {
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
            let client = self.client.clone();
            let request = self.as_mut().request();
            let next = request.send(client);
            self.as_mut().next().set(Some(next));
        }

        match self
            .as_mut()
            .next()
            .as_pin_mut()
            .expect("Assertion failed")
            .try_poll(ctx)
        {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(resp)) => Poll::Ready(Ok(resp)),
            Poll::Ready(Err(e)) => {
                self.as_mut().next().set(None);
                match self.as_mut().request().get_mut().next_wait(e) {
                    Ok(w) => {
                        self.as_mut().wait().set(Some(w));
                        self.poll_impl(ctx)
                    }
                    Err(e) => Poll::Ready(Err(e)),
                }
            }
        }
    }
}
