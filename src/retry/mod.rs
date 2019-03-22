mod error;
#[cfg(feature = "backoff-tokio")]
mod tokio;
mod util;

use std::pin::Pin;
use std::time::Duration;

use pin_utils::unsafe_pinned;

use crate::compat::{Poll, Waker};
use crate::request::{PagedRequest, RepeatableRequest, Request, RetriableRequest};
use crate::response::Response;

pub use self::{
    error::{BackoffError, RetryError},
    util::Retry,
};
pub use backoff::{backoff::Backoff, ExponentialBackoff};

#[cfg(feature = "backoff-tokio")]
pub use self::tokio::Delay;

pub struct WithBackoff<R, B, D> {
    inner: R,
    backoff: Pin<Box<dyn Fn() -> B + 'static>>,
    delay: Pin<Box<dyn Fn(Duration) -> D + 'static>>,
}

#[cfg(feature = "backoff-tokio")]
impl<R, B> WithBackoff<R, B, Delay> {
    fn with_retry<F>(req: R, backoff: F) -> Self
    where
        F: Fn() -> B + 'static,
    {
        WithBackoff {
            inner: req,
            backoff: Box::pin(backoff),
            delay: Box::pin(Delay::expires_in),
        }
    }
}

impl<R, B, D> Unpin for WithBackoff<R, B, D> where R: Unpin {}

impl<'a, R, B, D, C> Request<C> for &'a WithBackoff<R, B, D>
where
    R: RetriableRequest<C>,
    B: Backoff + Unpin,
    D: Response<Ok = (), Error = BackoffError>,
    C: Clone,
{
    type Ok = R::Ok;
    type Error = RetryError<R::Error>;
    type Response = RetriableResponse<'a, R, B, D, C>;

    fn into_response(self, client: C) -> Self::Response {
        self.send(client)
    }
}

impl<'a, R, B, D, C> RepeatableRequest<C> for &'a WithBackoff<R, B, D>
where
    R: RetriableRequest<C>,
    B: Backoff + Unpin,
    D: Response<Ok = (), Error = BackoffError>,
    C: Clone,
{
    fn send(&self, client: C) -> Self::Response {
        RetriableResponse {
            client,
            request: self,
            backoff: (self.backoff)(),
            next: None,
            wait: None,
        }
    }
}

impl<'a, R, B, D, C> RetriableRequest<C> for &'a WithBackoff<R, B, D>
where
    R: RetriableRequest<C>,
    B: Backoff + Unpin,
    D: Response<Ok = (), Error = BackoffError>,
    C: Clone,
{
    fn should_retry(&self, error: &Self::Error, next_interval: Duration) -> bool {
        if let Some(err) = error.as_inner() {
            self.inner.should_retry(err, next_interval)
        } else {
            false
        }
    }
}

pub struct RetriableResponse<'a, R, B, D, C>
where
    R: RetriableRequest<C>,
{
    client: C,
    request: &'a WithBackoff<R, B, D>,
    backoff: B,
    next: Option<R::Response>,
    wait: Option<D>,
}

impl<'a, R, B, D, C> RetriableResponse<'a, R, B, D, C>
where
    R: RetriableRequest<C>,
{
    unsafe_pinned!(request: &'a WithBackoff<R, B, D>);
    unsafe_pinned!(backoff: B);
    unsafe_pinned!(next: Option<R::Response>);
    unsafe_pinned!(wait: Option<D>);
}

impl<'a, R, B, D, C> Unpin for RetriableResponse<'a, R, B, D, C>
where
    R: RetriableRequest<C> + Unpin,
    R::Response: Unpin,
    B: Unpin,
    D: Unpin,
    C: Unpin,
{
}

impl<'a, R, B, D, C> Response for RetriableResponse<'a, R, B, D, C>
where
    R: RetriableRequest<C>,
    B: Backoff + Unpin,
    D: Response<Ok = (), Error = BackoffError>,
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
            let next = request.inner.send(self.client.clone());
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

impl<'a, R, B, D, C> RetriableResponse<'a, R, B, D, C>
where
    R: RetriableRequest<C>,
    B: Backoff + Unpin,
    D: Response<Ok = (), Error = BackoffError>,
    C: Clone,
{
    fn next_wait(mut self: Pin<&mut Self>, err: R::Error) -> Result<D, RetryError<R::Error>> {
        let next = self
            .as_mut()
            .backoff()
            .next_backoff()
            .ok_or_else(BackoffError::timeout)?;
        let err = RetryError::from_err(err);
        if self.as_ref().request.should_retry(&err, next) {
            Ok((self.as_ref().request.delay)(next))
        } else {
            Err(err)
        }
    }
}

#[cfg(all(test, feature = "std-future-test"))]
mod test {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use ::tokio::runtime::current_thread::block_on_all;
    use futures_util::{
        future::{self, FutureExt},
        try_future::TryFutureExt,
    };

    use super::*;
    use crate::prelude::*;
    use crate::response::{Response, ResponseStdFutureObj};

    #[derive(Debug, Default)]
    pub(crate) struct Numbers {
        current: AtomicUsize,
        end: usize,
    }

    impl Clone for Numbers {
        fn clone(&self) -> Self {
            Numbers {
                current: AtomicUsize::new(self.current.load(Ordering::SeqCst)),
                end: self.end,
            }
        }
    }

    type Resp = ResponseStdFutureObj<'static, usize, String>;

    impl<C> Request<C> for Numbers {
        type Ok = usize;
        type Error = String;
        type Response = Resp;

        fn into_response(self, client: C) -> Self::Response {
            self.send(client)
        }
    }

    impl<C> RepeatableRequest<C> for Numbers {
        fn send(&self, _client: C) -> Self::Response {
            let i = self.current.fetch_add(1, Ordering::SeqCst);
            if i < self.end {
                ResponseStdFutureObj::new(future::err(format!("{} tried", i)))
            } else {
                ResponseStdFutureObj::new(future::ok(i))
            }
        }
    }

    impl<C> RetriableRequest<C> for Numbers {
        fn should_retry(&self, error: &Self::Error, next_interval: Duration) -> bool {
            true
        }
    }

    fn block_on<R>(req: R) -> Result<R::Ok, R::Error>
    where
        R: Response + Unpin,
    {
        block_on_all(req.into_future().compat())
    }

    #[test]
    fn retry_simple() {
        let numbers = Numbers {
            current: AtomicUsize::new(1),
            end: 5,
        };
        let req = WithBackoff::with_retry(numbers, ExponentialBackoff::default);
        let r = &req;

        assert_eq!(block_on(r.send(())).unwrap(), 5);
    }
}
