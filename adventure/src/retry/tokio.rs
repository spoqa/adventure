use futures::prelude::*;
use tokio::time::{delay_until, Delay, Duration, Instant};

use super::{RetryError, Timer};

/// Provides a delayed response using [`tokio_timer`] crate.
#[derive(Clone, Default)]
pub struct TokioTimer;

impl Timer for TokioTimer {
    type Delay = future::ErrInto<future::NeverError<Delay>, RetryError>;

    fn expires_in(&mut self, duration: Duration) -> Self::Delay {
        let deadline = Instant::now() + duration;
        delay_until(deadline).never_error().err_into()
    }
}
