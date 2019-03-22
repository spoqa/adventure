use std::marker::PhantomData;

mod error;
#[cfg(feature = "backoff-tokio")]
mod tokio;
mod util;

use std::pin::Pin;
use std::time::Duration;

use pin_utils::unsafe_pinned;

#[cfg(feature = "backoff-tokio")]
use self::util::RetryBackoff;

use crate::compat::{Poll, Waker};
use crate::request::{PagedRequest, RepeatableRequest, Request, RetriableRequest};
use crate::response::Response;

pub use self::{
    error::{BackoffError, RetryError},
    util::Retry,
};
pub use backoff::{backoff::Backoff, ExponentialBackoff};

pub struct WithBackoff<R, T> {
    inner: R,
    _phantom: PhantomData<T>,
}

#[cfg(feature = "backoff-tokio")]
impl<R> WithBackoff<R, RetryBackoff> {
    fn with_retry(req: R) -> Self {
        WithBackoff {
            inner: req,
            _phantom: PhantomData,
        }
    }
}

impl<R, T> Unpin for WithBackoff<R, T> where R: Unpin {}

impl<'a, R, T, C> Request<C> for &'a WithBackoff<R, T>
where
    R: RetriableRequest<C> + Unpin,
    T: Retry + Unpin,
    C: Clone,
{
    type Ok = R::Ok;
    type Error = RetryError<R::Error>;
    type Response = RetriableResponse<'a, R, T, C>;

    fn into_response(self, client: C) -> Self::Response {
        self.send(client)
    }
}

impl<'a, R, T, C> RepeatableRequest<C> for &'a WithBackoff<R, T>
where
    R: RetriableRequest<C> + Unpin,
    T: Retry + Unpin,
    C: Clone,
{
    fn send(&self, client: C) -> Self::Response {
        RetriableResponse {
            client,
            request: self,
            retry: T::new(),
            next: None,
            wait: None,
        }
    }
}

impl<'a, R, T, C> RetriableRequest<C> for &'a WithBackoff<R, T>
where
    R: RetriableRequest<C> + Unpin,
    T: Retry + Unpin,
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

pub struct RetriableResponse<'a, R, T, C>
where
    R: RetriableRequest<C>,
    T: Retry,
{
    client: C,
    request: &'a WithBackoff<R, T>,
    retry: T,
    next: Option<R::Response>,
    wait: Option<T::Wait>,
}

impl<'a, R, T, C> RetriableResponse<'a, R, T, C>
where
    R: RetriableRequest<C>,
    T: Retry,
{
    unsafe_pinned!(request: &'a WithBackoff<R, T>);
    unsafe_pinned!(retry: T);
    unsafe_pinned!(next: Option<R::Response>);
    unsafe_pinned!(wait: Option<T::Wait>);
}

impl<'a, R, T, C> Unpin for RetriableResponse<'a, R, T, C>
where
    R: RetriableRequest<C> + Unpin,
    R::Response: Unpin,
    T: Retry + Unpin,
    T::Wait: Unpin,
    C: Unpin,
{
}

impl<'a, R, T, C> Response for RetriableResponse<'a, R, T, C>
where
    R: RetriableRequest<C> + Unpin,
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

impl<'a, R, T, C> RetriableResponse<'a, R, T, C>
where
    R: RetriableRequest<C> + Unpin,
    T: Retry + Unpin,
    C: Clone,
{
    fn next_wait(mut self: Pin<&mut Self>, err: R::Error) -> Result<T::Wait, RetryError<R::Error>> {
        let err = RetryError::from_err(err);
        let next = self.as_mut().retry().next_backoff()?;
        if self.as_ref().request.should_retry(&err, next) {
            Ok(self.as_ref().retry.wait(next))
        } else {
            Err(err)
        }
    }
}

#[cfg(all(test, feature = "std-future-test"))]
mod test {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use ::tokio::runtime::current_thread::block_on_all;
    use futures_util::{future, try_future::TryFutureExt};

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
        fn should_retry(&self, _error: &Self::Error, _next_interval: Duration) -> bool {
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
        let req = WithBackoff::with_retry(numbers);
        let r = &req;

        assert_eq!(block_on(r.send(())).unwrap(), 5);
    }
}
