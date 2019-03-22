mod backoff;

use std::error::Error as StdError;
use std::fmt::{self, Display};
use std::pin::Pin;

use pin_utils::unsafe_pinned;

use self::backoff::{BackoffError, Waiting};
use crate::compat::{Poll, Waker};
use crate::request::{PagedRequest, Request};
use crate::response::Response;

pub use ::backoff::{backoff::Backoff, ExponentialBackoff};

#[cfg(feature = "backoff-tokio")]
pub use self::timer::BackoffTimer;

#[derive(Debug)]
pub struct RetryError<E> {
    inner: RetryErrorKind<E>,
}

#[derive(Debug)]
enum RetryErrorKind<E> {
    Inner(E),
    Backoff(BackoffError),
}

impl<E: Display> Display for RetryError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use RetryErrorKind::*;
        match &self.inner {
            Inner(e) => e.fmt(f),
            Backoff(e) => e.fmt(f),
        }
    }
}

impl<E: StdError + 'static> StdError for RetryError<E> {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        use RetryErrorKind::*;
        match &self.inner {
            Inner(e) => Some(&*e),
            Backoff(e) => Some(&*e),
        }
    }
}

impl<E> RetryError<E> {
    pub fn into_inner(self) -> Option<E> {
        if let RetryErrorKind::Inner(e) = self.inner {
            Some(e)
        } else {
            None
        }
    }

    pub fn is_timeout(&self) -> bool {
        if let RetryErrorKind::Backoff(e) = &self.inner {
            e.is_timeout()
        } else {
            false
        }
    }

    pub fn is_shutdown(&self) -> bool {
        if let RetryErrorKind::Backoff(e) = &self.inner {
            e.is_shutdown()
        } else {
            false
        }
    }
}

impl<E> From<BackoffError> for RetryError<E> {
    fn from(e: BackoffError) -> Self {
        RetryError {
            inner: RetryErrorKind::Backoff(e),
        }
    }
}

pub trait Retry {
    type Backoff: Backoff;
    type Wait: Response<Ok = (), Error = BackoffError>;

    fn generate(&self) -> Self::Backoff;
    fn wait(backoff: &mut Self::Backoff) -> Option<Self::Wait>;
}

#[cfg(feature = "backoff-tokio")]
mod timer {
    use super::backoff::Delay;
    use super::*;

    pub struct BackoffTimer;

    impl Retry for BackoffTimer {
        type Backoff = ExponentialBackoff;
        type Wait = Delay;

        fn generate(&self) -> Self::Backoff {
            ExponentialBackoff::default()
        }

        fn wait(backoff: &mut Self::Backoff) -> Option<Self::Wait> {
            dbg!(backoff.next_backoff()).map(Delay::expires_in)
        }
    }
}

pub struct WithBackoff<R, T> {
    inner: R,
    retry: T,
}

impl<R, T> WithBackoff<R, T> {
    fn with_retry(req: R, retry: T) -> Self {
        WithBackoff { inner: req, retry }
    }
}

impl<R, T> From<R> for WithBackoff<R, T>
where
    T: Default,
{
    fn from(req: R) -> Self {
        Self::with_retry(req, Default::default())
    }
}

impl<R, T> Unpin for WithBackoff<R, T>
where
    R: Unpin,
    T: Unpin,
{
}

impl<R, T, C> Request<C> for WithBackoff<R, T>
where
    R: Request<C> + Clone + Unpin,
    T: Retry + Unpin,
    T::Backoff: Unpin,
    T::Wait: Unpin,
    C: Clone + Unpin,
{
    type Ok = R::Ok;
    type Error = RetryError<R::Error>;
    type Response = RetriableResponse<R, C, T>;

    fn send(&self, client: C) -> Self::Response {
        RetriableResponse {
            client,
            request: self.inner.clone(),
            timer: self.retry.generate(),
            next: None,
            wait: None,
        }
    }
}

impl<R, T, C> PagedRequest<C> for WithBackoff<R, T>
where
    R: PagedRequest<C> + Clone + Unpin,
    T: Retry + Unpin,
    T::Backoff: Unpin,
    T::Wait: Unpin,
    C: Clone + Unpin,
{
    fn advance(&mut self, response: &Self::Ok) -> bool {
        self.inner.advance(response)
    }
}

pub struct RetriableResponse<R, C, T>
where
    R: Request<C>,
    T: Retry,
{
    client: C,
    request: R,
    timer: T::Backoff,
    next: Option<R::Response>,
    wait: Option<Waiting<T::Wait>>,
}

impl<R, C, T> RetriableResponse<R, C, T>
where
    R: Request<C>,
    T: Retry,
{
    unsafe_pinned!(timer: T::Backoff);

    unsafe_pinned!(next: Option<R::Response>);

    unsafe_pinned!(wait: Option<Waiting<T::Wait>>);
}

impl<R, C, T> Unpin for RetriableResponse<R, C, T>
where
    R: Request<C> + Unpin,
    C: Unpin,
    T: Retry,
    T::Backoff: Unpin,
    T::Wait: Unpin,
{
}

impl<R, C, T> Response for RetriableResponse<R, C, T>
where
    R: Request<C>,
    C: Clone,
    T: Retry,
    T::Backoff: Unpin,
    T::Wait: Unpin,
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
                let w = T::wait(self.as_mut().timer().get_mut());
                self.as_mut().next().set(None);
                self.as_mut().wait().set(Some(w.into()));
                self.poll(waker)
            }
        }
    }
}

#[cfg(all(test, feature = "std-future-test"))]
mod test {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use futures_util::{
        future::{self, FutureExt},
        try_future::TryFutureExt,
    };
    use tokio::runtime::current_thread::block_on_all;

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

    impl Request<()> for Numbers {
        type Ok = usize;
        type Error = String;
        type Response = Resp;

        fn send(&self, client: ()) -> Self::Response {
            let i = self.current.fetch_add(1, Ordering::SeqCst);
            if i < self.end {
                ResponseStdFutureObj::new(future::err(format!("{} tried", i)))
            } else {
                ResponseStdFutureObj::new(future::ok(i))
            }
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
        let req = WithBackoff::with_retry(numbers, BackoffTimer);

        assert_eq!(block_on(req.send(())).unwrap(), 5);
    }
}
