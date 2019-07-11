use core::time::Duration;

pub use backoff::{backoff::Backoff, ExponentialBackoff as ExponentialBackoffImpl, SystemClock};

#[derive(Default)]
pub struct ExponentialBackoff {
    inner: ExponentialBackoffImpl,
}

impl AsRef<ExponentialBackoffImpl> for ExponentialBackoff {
    fn as_ref(&self) -> &ExponentialBackoffImpl {
        &self.inner
    }
}

impl AsMut<ExponentialBackoffImpl> for ExponentialBackoff {
    fn as_mut(&mut self) -> &mut ExponentialBackoffImpl {
        &mut self.inner
    }
}

impl Backoff for ExponentialBackoff {
    fn reset(&mut self) {
        self.inner.reset()
    }
    fn next_backoff(&mut self) -> Option<Duration> {
        self.inner.next_backoff()
    }
}

impl Clone for ExponentialBackoff {
    fn clone(&self) -> Self {
        let inner = ExponentialBackoffImpl {
            clock: SystemClock::default(),
            ..self.inner
        };
        ExponentialBackoff { inner }
    }
}
