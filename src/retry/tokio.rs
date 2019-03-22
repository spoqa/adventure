use std::pin::Pin;
use std::time::{Duration, Instant};

use tokio_timer::Delay as DelayImpl;

use super::{Backoff, BackoffError, ExponentialBackoff, Retry};
use crate::compat::{Compat, Poll, Waker};
use crate::response::Response;

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

pub struct Delay {
    inner: Compat<DelayImpl>,
}

impl Delay {
    pub(crate) fn expires_in(duration: Duration) -> Self {
        let deadline = Instant::now() + duration;
        let delay = DelayImpl::new(deadline);
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
