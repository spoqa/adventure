use std::error::Error as StdError;
use std::fmt::{self, Display};
use std::pin::Pin;
use std::time::Duration;

use crate::compat::{Poll, Waker};
use crate::response::Response;

#[cfg(feature = "backoff-tokio")]
pub use self::impl_std::Delay;

#[derive(Debug)]
pub struct BackoffError {
    inner: BackoffErrorKind,
}

#[derive(Debug)]
enum BackoffErrorKind {
    Timeout,
    TimerShutdown,
}

impl Display for BackoffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use BackoffErrorKind::*;
        match self.inner {
            Timeout => "Timeout reached".fmt(f),
            TimerShutdown => "Timer has gone".fmt(f),
        }
    }
}

impl StdError for BackoffError {}

impl BackoffError {
    const fn timeout() -> Self {
        BackoffError {
            inner: BackoffErrorKind::Timeout,
        }
    }
    const fn shutdown() -> Self {
        BackoffError {
            inner: BackoffErrorKind::TimerShutdown,
        }
    }

    pub(crate) fn is_timeout(&self) -> bool {
        if let BackoffErrorKind::Timeout = self.inner {
            true
        } else {
            false
        }
    }

    pub(crate) fn is_shutdown(&self) -> bool {
        if let BackoffErrorKind::TimerShutdown = self.inner {
            true
        } else {
            false
        }
    }
}

pub trait Wait: Response<Ok = (), Error = BackoffError> + Unpin {
    fn expires_in(duration: Duration) -> Self;
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

#[cfg(feature = "backoff-tokio")]
mod impl_std {
    use std::time::{Duration, Instant};

    use crate::compat::{Compat, Poll, Waker};

    use super::*;

    pub struct Delay {
        inner: Compat<tokio_timer::Delay>,
    }

    impl Delay {
        pub(crate) fn expires_in(duration: Duration) -> Self {
            let deadline = Instant::now() + duration;
            let delay = tokio_timer::Delay::new(deadline);
            Delay {
                inner: Compat::new(delay),
            }
        }
    }

    impl Response for Delay {
        type Ok = ();
        type Error = BackoffError;

        fn poll(mut self: Pin<&mut Self>, w: &Waker) -> Poll<Result<Self::Ok, Self::Error>> {
            let r = match Response::poll(Pin::new(&mut self.inner), w) {
                Poll::Pending => {
                    return Poll::Pending;
                }
                Poll::Ready(Err(ref e)) if e.is_shutdown() => Err(BackoffError::shutdown()),
                Poll::Ready(_) => Ok(()),
            };
            Poll::Ready(r)
        }
    }
}
