use std::pin::Pin;
use std::time::{Duration, Instant};

use tokio_timer::Delay as DelayImpl;

use super::BackoffError;
use crate::compat::{Compat, Poll, Waker};
use crate::response::Response;

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
