use std::pin::Pin;
use std::time::{Duration, Instant};

use tokio_timer::Delay as DelayImpl;

use super::{RetryError, Timer};
use crate::response::Response;
use crate::task::{Compat, Poll, Waker};

/// Provides a delayed response using [`tokio_timer`] crate.
#[derive(Clone, Default)]
pub struct TokioTimer;

/// A response that completes at a specified instant in time.
pub struct Delay {
    inner: Compat<DelayImpl>,
}

impl Timer for TokioTimer {
    type Delay = Delay;

    fn expires_in(&mut self, duration: Duration) -> Self::Delay {
        let deadline = Instant::now() + duration;
        let delay = DelayImpl::new(deadline);
        Delay {
            inner: Compat::new(delay),
        }
    }
}

impl Response for Delay {
    type Ok = ();
    type Error = RetryError;

    fn poll(mut self: Pin<&mut Self>, w: &Waker) -> Poll<Result<Self::Ok, Self::Error>> {
        let r = match Response::poll(Pin::new(&mut self.inner), w) {
            Poll::Pending => {
                return Poll::Pending;
            }
            Poll::Ready(Err(ref e)) if e.is_shutdown() => Err(RetryError::shutdown()),
            Poll::Ready(_) => Ok(()),
        };
        Poll::Ready(r)
    }
}
